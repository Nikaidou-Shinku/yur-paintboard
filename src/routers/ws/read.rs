use std::sync::Arc;

use uuid::Uuid;
use chrono::Local;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
use axum::extract::ws::Message;

use crate::{AppState, channel::ChannelMsg};
use yur_paintboard::{
  consts::{WIDTH, HEIGHT},
  entities::{prelude::*, session, board},
  pixel::{color_to_hex, Pixel},
};

pub async fn ws_read(
  state: Arc<AppState>,
  uid: Option<i32>,
  msg: Message,
) -> Option<i32> {
  let msg = msg.into_data();

  let (opt, data) = msg.split_first().unwrap();

  match opt {
    0xff => { // Auth
      if uid.is_some() {
        return uid;
      }

      handle_auth(state, data).await
    },
    0xfe => { // Paint
      if let Some(uid) = uid {
        handle_paint(state, uid, data).await;
      }

      None
    },
    _ => { None },
  }
}

async fn handle_auth(
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

  match session {
    None => None,
    Some(session) => {
      let uid = session.uid;

      let user_ws = state.user_ws.lock().unwrap();

      if let Some(user_ws) = user_ws.get(&uid) {
        if user_ws.is_some() {
          return None; // already connected
        }
      }

      Some(uid)
    },
  }
}

async fn handle_paint(
  state: Arc<AppState>,
  uid: i32,
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

  { // check interval
    let now = Local::now();
    let mut user_paint = state.user_paint.lock().unwrap();

    if let Some(last_paint) = user_paint.get(&uid) {
      // TODO(config)
      if (now - *last_paint) < chrono::Duration::milliseconds(100) {
        return;
      }
    }

    user_paint.insert(uid, now);
  }

  let new_pixel = board::Model {
    x: x.into(),
    y: y.into(),
    color: color_to_hex(color),
    uid,
    time: Local::now(),
  };

  let idx = x * HEIGHT + y;

  {
    let mut pixel = state.board[idx as usize].lock().unwrap();

    let same = pixel.color == new_pixel.color;

    *pixel = new_pixel;

    if same { // Same color
      return;
    }
  }

  state.sender
    .send(ChannelMsg::Paint(Pixel { x, y, color })).unwrap();
}
