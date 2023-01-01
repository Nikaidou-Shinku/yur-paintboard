use std::sync::Arc;

use uuid::Uuid;
use chrono::Local;
use serde::{Deserialize, Serialize};
use futures::{StreamExt, SinkExt};
use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
use axum::{extract::{State, WebSocketUpgrade, ws::{WebSocket, Message}}, response::Response};

use crate::{AppState, channel::ChannelMsg, paint::{Paint, color_to_hex, hex_to_bin}};
use yur_paintboard::entities::{prelude::*, session, board};

pub async fn ws(
  State(state): State<Arc<AppState>>,
  ws: WebSocketUpgrade,
) -> Response {
  ws.on_upgrade(|socket| handle_ws(state, socket))
}

async fn handle_ws(
  state: Arc<AppState>,
  socket: WebSocket,
) {
  let (mut ws_out, mut ws_in) = socket.split();

  let uid: i32;

  loop {
    let msg = ws_in.next().await;

    if msg.is_none() {
      return;
    }

    let msg = msg.unwrap();

    if msg.is_err() {
      continue;
    }

    let msg = msg.unwrap();

    let res = handle_ws_in(state.clone(), None, msg).await;

    if let Some(res) = res {
      uid = res;
      break;
    }
  }

  ws_out
    .send(
      Message::Text(r#"{"type":"Auth","code":0}"#.to_string())
    ).await.unwrap();

  let board = state.board.iter()
    .map(|pixel| hex_to_bin(&pixel.lock().unwrap().color))
    .flatten()
    .collect::<Vec<u8>>();

  ws_out
    .send(
      Message::Binary(board)
    ).await.unwrap();

  let session = Uuid::new_v4();

  let read_task = async {
    loop {
      let msg = ws_in.next().await;

      if msg.is_none() {
        break;
      }

      let msg = msg.unwrap();

      if msg.is_err() {
        continue;
      }

      let msg = msg.unwrap();

      handle_ws_in(state.clone(), Some(uid), msg).await;
    }

    state.sender.send(ChannelMsg::Close(session)).unwrap();

    println!("[RD] {session} closed.");
  };

  let write_task = async {
    let mut receier = state.sender.subscribe();

    loop {
      let msg = receier.recv().await;

      if msg.is_err() {
        continue;
      }

      let msg = msg.unwrap();

      match msg {
        ChannelMsg::Close(event) => {
          if event == session {
            break;
          }
        },
        ChannelMsg::Paint(paint) => {
          let msg = serde_json::to_string(&WsMsg::Paint(paint)).unwrap();
          let res = ws_out.send(Message::Text(msg)).await;

          if res.is_err() {
            eprintln!("[WT] {session} send error!");
            break;
          }
        },
      }
    }

    println!("[WT] {session} closed.");
  };

  futures::future::join(read_task, write_task).await;
}

#[derive(Deserialize, Serialize)]
struct WsAuthMsg {
  token: Uuid,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
enum WsMsg {
  Auth(WsAuthMsg),
  Paint(Paint),
}

async fn handle_ws_in(
  state: Arc<AppState>,
  uid: Option<i32>,
  msg: Message,
) -> Option<i32> {
  let msg = msg.to_text();

  if msg.is_err() {
    return None;
  }

  let msg = msg.unwrap();
  let msg = serde_json::from_str(msg);

  if msg.is_err() {
    return None;
  }

  let msg: WsMsg = msg.unwrap();

  match msg {
    WsMsg::Auth(WsAuthMsg { token }) => {
      return ws_auth(state, token).await;
    },
    WsMsg::Paint(paint) => {
      if uid.is_none() { // Not auth
        return None;
      }

      let idx = paint.x * 600 + paint.y;

      if idx < 0 || idx >= 600000 { // Out of board
        return None;
      }

      let new_paint = board::Model {
        x: paint.x,
        y: paint.y,
        color: color_to_hex(paint.c),
        uid: uid.unwrap(),
        time: Local::now(),
      };

      {
        let mut pixel = state.board[idx as usize].lock().unwrap();

        let same = pixel.color == new_paint.color;

        *pixel = new_paint;

        if same { // Same color
          return None;
        }
      }

      state.sender
        .send(ChannelMsg::Paint(paint)).unwrap();

      return None;
    },
  }
}

async fn ws_auth(
  state: Arc<AppState>,
  token: Uuid,
) -> Option<i32> {
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
