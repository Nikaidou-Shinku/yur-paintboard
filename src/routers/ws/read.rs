use std::sync::Arc;

use uuid::Uuid;
use chrono::Local;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, ActiveValue};
use axum::extract::ws::{Message, WebSocket};

use yur_paintboard::{
  consts::{WIDTH, HEIGHT},
  entities::{prelude::*, session, board, paint},
  pixel::{color_to_hex, Pixel, hex_to_bin},
};
use crate::AppState;
use super::WsState;

pub async fn ws_read(
  state: Arc<AppState>,
  socket: &mut WebSocket,
  ws_state: &mut WsState,
  msg: Option<Result<Message, axum::Error>>,
) -> bool {
  if msg.is_none() {
    return true;
  }
  let msg = msg.unwrap();

  if msg.is_err() {
    return false;
  }
  let msg = msg.unwrap();

  let msg = msg.into_data();
  let msg = msg.split_first();

  if msg.is_none() {
    return false;
  }
  let (opt, data) = msg.unwrap();

  match opt {
    0xff => { // Auth
      if ws_state.uid.is_none() {
        ws_state.uid = handle_auth(state, data).await;

        match ws_state.uid {
          Some(uid) => {
            tracing::Span::current().record("uid", uid);

            let res = socket.send(Message::Binary(vec![0xfc])).await; // auth success
            if res.is_err() {
              return true;
            }

            tracing::info!("Authenticated.");
          },
          None => {
            let res = socket.send(Message::Binary(vec![0xfd])).await; // auth failed
            if res.is_err() {
              return true;
            }
          },
        }
      }
    },
    0xfe => { // Paint
      // TODO(config)
      if ws_state.quick_paint > 3 {
        return true;
      }

      if let Some(_) = ws_state.uid {
        handle_paint(state, ws_state, data).await;
      }
    },
    0xf9 => { // Board
      if let Some(_) = ws_state.uid {
        let board = get_board(state);

        ws_state.readonly = false;
  
        let res = socket.send(Message::Binary(board)).await;
        if res.is_err() {
          return true;
        }
  
        tracing::info!("Sent board.");
      }
    },
    0xf7 => { // Pong
      ws_state.get_pong = true;
    },
    _ => {
      tracing::warn!("Unknown message!");
      ws_state.trash_pack += 1;

      // TODO(config)
      if ws_state.trash_pack > 0 {
        return true;
      }
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
  ws_state: &mut WsState,
  data: &[u8],
) {
  if data.len() != 7 {
    return;
  }

  let x = u16::from_le_bytes([data[0], data[1]]);

  if x >= WIDTH {
    return;
  }

  let y = u16::from_le_bytes([data[2], data[3]]);

  if y >= HEIGHT {
    return;
  }

  let color = (data[4], data[5], data[6]);

  let uid = ws_state.uid.unwrap();

  let now = Local::now();

  { // check interval
    let mut user_paint = state.user_paint.lock();

    if let Some(last_paint) = user_paint.get(&uid) {
      // TODO(config)
      if (now - *last_paint) < chrono::Duration::milliseconds(500) {
        ws_state.quick_paint += 1;
        return;
      } else {
        ws_state.quick_paint = 0;
      }
    }

    user_paint.insert(uid, now);
  }

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
