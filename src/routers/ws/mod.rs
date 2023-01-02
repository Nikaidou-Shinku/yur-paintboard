mod read;
mod write;

use std::sync::Arc;

use uuid::Uuid;
use futures::{StreamExt, SinkExt};
use axum::{extract::{State, WebSocketUpgrade, ws::{WebSocket, Message}}, response::Response};

use self::{read::ws_read, write::ws_write};
use crate::{AppState, channel::ChannelMsg};
use yur_paintboard::pixel::hex_to_bin;

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

  loop { // wait for auth
    let msg = ws_in.next().await;

    if msg.is_none() {
      return;
    }

    let msg = msg.unwrap();

    if msg.is_err() {
      continue;
    }

    let msg = msg.unwrap();

    let res = ws_read(state.clone(), None, msg).await;

    if let Some(res) = res {
      uid = res;
      break;
    }

    let res = ws_out
      .send(
        Message::Binary(vec![0xfd]) // auth failed
      ).await;

    if res.is_err() {
      return;
    }
  }

  ws_out
    .send(
      Message::Binary(vec![0xfc]) // auth success
    ).await.unwrap();

  let ws_session = Uuid::new_v4();

  { // user connected ws
    let mut user_ws = state.user_ws.lock().unwrap();
    user_ws.insert(uid, Some(ws_session));
  }

  println!("[WS] {uid} authenticated.");

  let board = state.board.iter()
    .map(|pixel| hex_to_bin(&pixel.lock().unwrap().color))
    .flatten()
    .collect::<Vec<u8>>();

  // TODO: maybe more elegant way to do this
  let mut board = zstd::encode_all(board.as_slice(), 0).unwrap();
  board.insert(0, 0xfb);

  println!("[WS] parse board for {uid}.");

  ws_out
    .send(
      Message::Binary(board)
    ).await.unwrap();

  println!("[WS] send board for {uid}.");

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

      ws_read(state.clone(), Some(uid), msg).await;
    }

    state.sender.send(ChannelMsg::Close(ws_session)).unwrap();

    println!("[RD] {ws_session} closed.");
  };

  let write_task = ws_write(state.clone(), ws_session, ws_out);

  futures::future::join(read_task, write_task).await;

  { // user disconnected ws
    let mut user_ws = state.user_ws.lock().unwrap();
    user_ws.insert(uid, None);
  }
}
