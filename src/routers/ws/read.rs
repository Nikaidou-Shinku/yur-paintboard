use std::sync::Arc;

use uuid::Uuid;
use chrono::Local;
use futures::{stream::SplitSink, SinkExt};
use parking_lot::Mutex;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, ActiveValue};
use axum::extract::ws::{Message, WebSocket};

use yur_paintboard::{
  consts::{WIDTH, HEIGHT},
  entities::{prelude::*, session, board, paint},
  pixel::{color_to_hex, Pixel, hex_to_bin},
};
use crate::AppState;
use super::WsState;

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
    0xff => { // Auth
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

          let res = ws_out.lock().await
            .send(Message::Binary(vec![0xfc])).await; // auth success
          if res.is_err() {
            tracing::warn!("Error sending auth result, closing...");
            return true;
          }

          tracing::info!("Authenticated.");
        },
        None => {
          tracing::warn!("Auth failed!");

          let res = ws_out.lock().await
            .send(Message::Binary(vec![0xfd])).await; // auth failed
          if res.is_err() {
            tracing::warn!("Error sending auth result, closing...");
            return true;
          }

          ws_state.lock().trash_pack += 1;
        },
      }
    },
    0xfe => { // Paint
      if ws_state.lock().uid.is_none() {
        tracing::warn!("Paint without auth!");
        ws_state.lock().trash_pack += 1;
        return false;
      }

      handle_paint(state, ws_state, data).await;
    },
    0xf9 => { // Board
      tracing::info!("Request for board.");

      if ws_state.lock().uid.is_none() {
        tracing::warn!("Get board without auth!");
        ws_state.lock().trash_pack += 1;
        return false;
      }

      if !ws_state.lock().readonly { // refuse to send board twice
        tracing::warn!("Duplicate board request, closing...");
        return true;
      }

      let board = get_board(state);

      ws_state.lock().readonly = false;

      let res = ws_out.lock().await
        .send(Message::Binary(board)).await;
      if res.is_err() {
        tracing::warn!("Error sending board, closing...");
        return true;
      }

      tracing::info!("Sent board.");
    },
    0xf7 => { // Pong
      tracing::info!("Pong!");
      ws_state.lock().get_pong = true;
    },
    _ => {
      tracing::warn!("Unknown message!");
      ws_state.lock().trash_pack += 1;
    },
  }

  return false;
}

pub async fn handle_auth(
  state: Arc<AppState>,
  data: &[u8],
) -> Option<i32> {
  let token = Uuid::from_slice(data);

  if token.is_err() {
    return None;
  }

  let token = token.unwrap();

  let session = Session::find()
    .filter(session::Column::PaintToken.eq(token))
    .one(&state.db).await;

  if session.is_err() {
    return None;
  }

  let session = session.unwrap();

  session.map(|session| session.uid)
}

pub async fn handle_paint(
  state: Arc<AppState>,
  ws_state: &Mutex<WsState>,
  data: &[u8],
) {
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

  // check interval
  let last_paint = {
    let user_paint = state.user_paint.lock();
    user_paint.get(&uid).map(|item| item.to_owned())
  };

  if let Some(last_paint) = last_paint {
    let mut ws_state = ws_state.lock();
    // TODO(config)
    if (now - last_paint) < chrono::Duration::milliseconds(500) {
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

  let same = { // same color
    let mut pixel = state.board
      .get(&(x, y)).unwrap()
      .lock();

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
    state.sender
      .send(Pixel { x, y, color }).unwrap();
  }
}

pub fn get_board(
  state: Arc<AppState>,
) -> Vec<u8> {
  let max_len = WIDTH as usize * HEIGHT as usize * 3;
  let mut board = Vec::with_capacity(max_len);

  for x in 0..WIDTH {
    for y in 0..HEIGHT {
      let pixel = state.board
        .get(&(x, y)).unwrap()
        .lock();
      let pixel_bytes = hex_to_bin(&pixel.color);
      board.extend_from_slice(&pixel_bytes);
    }
  }

  // TODO(config): compress level
  let mut board = zstd::encode_all(board.as_slice(), 19).unwrap();
  board.insert(0, 0xfb);

  board
}
