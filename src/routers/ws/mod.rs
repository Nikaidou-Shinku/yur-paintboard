mod read;
mod write;

use std::sync::Arc;

use uuid::Uuid;
use futures::{StreamExt, SinkExt};
use axum::{extract::{State, WebSocketUpgrade, ws::{WebSocket, Message}}, response::Response};

use tracing::Instrument;

use yur_paintboard::{pixel::hex_to_bin, consts::{WIDTH, HEIGHT}};
use crate::{AppState, channel::ChannelMsg};
use self::{read::ws_read, write::ws_write};

pub async fn ws(
  State(state): State<Arc<AppState>>,
  ws: WebSocketUpgrade,
) -> Response {
  ws.on_upgrade(|socket| handle_ws(state, socket))
}

#[tracing::instrument(name = "ws", skip_all, fields(uid))]
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

    let res = ws_out.send(Message::Binary(vec![0xfd])).await; // auth failed

    if res.is_err() {
      return;
    }
  }

  tracing::Span::current()
    .record("uid", uid);

  let res = ws_out.send(Message::Binary(vec![0xfc])).await; // auth success

  if res.is_err() {
    return;
  }

  tracing::info!("Authenticated.");

  // TODO: only parse board when needed
  // TODO: maybe more elegant way to do this
  let mut board = vec![];

  for x in 0..WIDTH {
    for y in 0..HEIGHT {
      let pixel = state.board
        .get(&(x, y)).unwrap()
        .lock().unwrap();
      board.extend_from_slice(&hex_to_bin(&pixel.color));
    }
  }

  // TODO(config): compress level
  let mut board = zstd::encode_all(board.as_slice(), 0).unwrap();
  board.insert(0, 0xfb);

  tracing::info!("Parse board.");

  ws_out.send(Message::Binary(board)).await.unwrap();

  tracing::info!("Send board.");

  let ws_session = Uuid::new_v4();

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

    tracing::info!("Closed.");
  };

  let read_task = read_task.instrument(tracing::info_span!("read"));

  let write_task = ws_write(state.clone(), ws_session, ws_out);

  futures::future::join(read_task, write_task).await;
}
