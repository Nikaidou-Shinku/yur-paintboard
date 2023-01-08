use std::sync::Arc;

use uuid::Uuid;
use chrono::Local;
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, ActiveValue};
use axum::extract::ws::Message;

use yur_paintboard::{
  consts::{WIDTH, HEIGHT},
  entities::{prelude::*, session, board, paint},
  pixel::{color_to_hex, Pixel},
};
use crate::{AppState, channel::ChannelMsg};

pub async fn ws_read(
  state: Arc<AppState>,
  uid: Option<i32>,
  ws_session: Option<Uuid>,
  msg: Message,
) -> Option<i32> {
  let msg = msg.into_data();
  let msg = msg.split_first();

  if msg.is_none() {
    return None;
  }

  let (opt, data) = msg.unwrap();

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
    0xf9 => { // Board
      if let Some(ws_session) = ws_session {
        handle_board(state, ws_session);
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
    Some(session) => Some(session.uid),
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

  let now = Local::now();

  { // check interval
    let mut user_paint = state.user_paint.lock();

    if let Some(last_paint) = user_paint.get(&uid) {
      // TODO(config)
      if (now - *last_paint) < chrono::Duration::milliseconds(500) {
        return;
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
      .send(ChannelMsg::Paint(Pixel { x, y, color })).unwrap();
  }
}

fn handle_board(
  state: Arc<AppState>,
  ws_session: Uuid,
) {
  state.sender
    .send(ChannelMsg::Board(ws_session)).unwrap();
}
