use std::sync::Arc;

use axum::extract::ws::{Message, WebSocket};
use chrono::Local;
use futures::{stream::SplitSink, SinkExt};
use jsonwebtoken::{decode, Algorithm, Validation};
use parking_lot::Mutex;
use sea_orm::ActiveValue;
use serde::Deserialize;

use super::WsState;
use crate::AppState;
use yur_paintboard::{
  consts::{BEGIN_TIME, END_TIME, HEIGHT, WIDTH},
  entities::{board, paint},
  pixel::{color_to_hex, hex_to_bin, Pixel},
};

pub async fn handle_read(
  state: Arc<AppState>,
  ws_out: &tokio::sync::Mutex<SplitSink<WebSocket, Message>>,
  ws_state: &Mutex<WsState>,
  msg: Option<Result<Message, axum::Error>>,
) -> bool {
  if msg.is_none() {
    tracing::info!("Received empty message, closing...");
    return true;
  }
  let msg = msg.unwrap();

  if msg.is_err() {
    tracing::warn!("Error receiving message");
    return false;
  }
  let msg = msg.unwrap();

  if let Message::Close(_) = msg {
    tracing::info!("Client closed connection, closing...");
    return true;
  }

  let msg = msg.into_data();
  let msg = msg.split_first();

  if msg.is_none() {
    tracing::warn!("Received empty data");
    return false;
  }
  let (opt, data) = msg.unwrap();

  match opt {
    0xff => {
      // Auth
      let mut uid = ws_state.lock().uid;

      if uid.is_some() {
        tracing::warn!("Duplicated auth!");
        ws_state.lock().trash_pack += 1;
        return false;
      }

      uid = handle_auth(state, data).await;

      match uid {
        Some(uid) => {
          ws_state.lock().uid = Some(uid);
          tracing::Span::current().record("uid", uid);

          let res = ws_out.lock().await.send(Message::Binary(vec![0xfc])).await; // auth success
          if res.is_err() {
            tracing::warn!("Error sending auth result, closing...");
            return true;
          }

          tracing::info!("Authenticated.");
        }
        None => {
          tracing::warn!("Auth failed!");

          let res = ws_out.lock().await.send(Message::Binary(vec![0xfd])).await; // auth failed
          if res.is_err() {
            tracing::warn!("Error sending auth result, closing...");
            return true;
          }

          ws_state.lock().trash_pack += 1;
        }
      }
    }
    0xfe => {
      // Paint
      if ws_state.lock().uid.is_none() {
        tracing::warn!("Paint without auth!");
        ws_state.lock().trash_pack += 1;
        return false;
      }

      handle_paint(state, ws_state, data).await;
    }
    0xf9 => {
      // Board
      tracing::info!("Request for board.");

      if ws_state.lock().uid.is_none() {
        tracing::warn!("Get board without auth!");
        ws_state.lock().trash_pack += 1;
        return false;
      }

      if !ws_state.lock().readonly {
        // refuse to send board twice
        tracing::warn!("Duplicate board request, closing...");
        return true;
      }

      let board = get_board(state);

      ws_state.lock().readonly = false;

      let res = ws_out.lock().await.send(Message::Binary(board)).await;
      if res.is_err() {
        tracing::warn!("Error sending board, closing...");
        return true;
      }

      tracing::info!("Sent board.");
    }
    0xf7 => {
      // Pong
      tracing::info!("Pong!");
      ws_state.lock().get_pong = true;
    }
    _ => {
      tracing::warn!("Unknown message!");
      ws_state.lock().trash_pack += 1;
    }
  }

  return false;
}

#[derive(Deserialize)]
struct Claims {
  #[allow(dead_code)]
  exp: usize,
  uid: i32,
}

#[tracing::instrument(name = "auth", skip_all)]
pub async fn handle_auth(state: Arc<AppState>, data: &[u8]) -> Option<i32> {
  let raw_token = std::str::from_utf8(data);

  if raw_token.is_err() {
    tracing::warn!("Error decoding token!");
    return None;
  }
  let raw_token = raw_token.unwrap();

  let token = decode::<Claims>(raw_token, &state.pubkey, &Validation::new(Algorithm::EdDSA));

  if let Err(err) = token {
    tracing::warn!(token = raw_token, "Invalid token: {err}");
    return None;
  }
  let token = token.unwrap();

  Some(token.claims.uid)
}

#[tracing::instrument(name = "paint", skip_all)]
pub async fn handle_paint(state: Arc<AppState>, ws_state: &Mutex<WsState>, data: &[u8]) {
  if data.len() != 7 {
    tracing::warn!(len = data.len(), "Invalid paint data!");
    ws_state.lock().trash_pack += 1;
    return;
  }

  let x = u16::from_le_bytes([data[0], data[1]]);

  if x >= WIDTH {
    tracing::warn!(x, "Invalid paint data!");
    ws_state.lock().trash_pack += 1;
    return;
  }

  let y = u16::from_le_bytes([data[2], data[3]]);

  if y >= HEIGHT {
    tracing::warn!(y, "Invalid paint data!");
    ws_state.lock().trash_pack += 1;
    return;
  }

  let color = (data[4], data[5], data[6]);

  let uid = ws_state.lock().uid.unwrap();

  let now = Local::now();

  if now.time() < *BEGIN_TIME || now.time() > *END_TIME {
    tracing::warn!("Painting outside the specified time");
    ws_state.lock().trash_pack += 1;
    return;
  }

  // check interval
  let last_paint = {
    let user_paint = state.user_paint.lock();
    user_paint.get(&uid).map(|item| item.to_owned())
  };

  if let Some(last_paint) = last_paint {
    let mut ws_state = ws_state.lock();
    // TODO(config)
    if (now - last_paint) < chrono::Duration::milliseconds(100) {
      tracing::info!("Quick paint");
      ws_state.quick_paint += 1;
      return;
    } else {
      ws_state.quick_paint = 0;
    }
  }

  state.user_paint.lock().insert(uid, now);

  let hex_color = color_to_hex(color);

  let new_pixel = board::Model {
    x: x.into(),
    y: y.into(),
    color: hex_color.clone(),
    uid,
    time: now,
  };

  let same = {
    // same color
    let mut pixel = state.board.get(&(x, y)).unwrap().lock();

    let same = pixel.color == new_pixel.color;

    *pixel = new_pixel;

    same
  };

  let new_action = paint::ActiveModel {
    x: ActiveValue::set(x.into()),
    y: ActiveValue::set(y.into()),
    color: ActiveValue::set(hex_color),
    uid: ActiveValue::set(uid),
    time: ActiveValue::set(now),
    ..Default::default()
  };

  state.actions.lock().push(new_action);

  if !same {
    state.sender.send(Pixel { x, y, color }).unwrap();
  }
}

pub fn get_board(state: Arc<AppState>) -> Vec<u8> {
  let max_len = WIDTH as usize * HEIGHT as usize * 3;
  let mut board = Vec::with_capacity(max_len);

  for x in 0..WIDTH {
    for y in 0..HEIGHT {
      let pixel = state.board.get(&(x, y)).unwrap().lock();
      let pixel_bytes = hex_to_bin(&pixel.color);
      board.extend_from_slice(&pixel_bytes);
    }
  }

  // TODO(config): compress level
  let mut board = zstd::encode_all(board.as_slice(), 19).unwrap();
  board.insert(0, 0xfb);

  board
}
