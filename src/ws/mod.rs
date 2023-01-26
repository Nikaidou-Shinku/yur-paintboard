mod read;

use std::{sync::Arc, time::Duration};

use axum::{
  extract::{
    ws::{Message, WebSocket},
    State, WebSocketUpgrade,
  },
  response::Response,
};
use futures::{
  stream::{SplitSink, SplitStream},
  SinkExt, StreamExt,
};
use parking_lot::Mutex;

use crate::AppState;
use read::handle_read;
use yur_paintboard::pixel::Pixel;

pub async fn ws(State(state): State<Arc<AppState>>, ws: WebSocketUpgrade) -> Response {
  ws.on_upgrade(|socket| handle_ws(state, socket))
}

pub struct WsState {
  uid: Option<i32>,
  readonly: bool,
  get_pong: bool,
  quick_paint: u8,
  trash_pack: u8,
}

#[tracing::instrument(name = "ws", skip_all, fields(uid))]
async fn handle_ws(state: Arc<AppState>, socket: WebSocket) {
  let (ws_out, ws_in) = socket.split();
  let ws_out = tokio::sync::Mutex::new(ws_out);
  let ws_state = WsState {
    uid: None,
    readonly: true,
    get_pong: false,
    quick_paint: 0,
    trash_pack: 0,
  };
  let ws_state = Mutex::new(ws_state);
  let ws_paints = Mutex::new(vec![]);

  tokio::select! {
    _ = ws_read(ws_in, &ws_out, state.clone(), &ws_state) => { },
    _ = recv_paint(state, &ws_state, &ws_paints) => { },
    _ = ws_write(&ws_out, &ws_paints) => { },
    _ = heartbeat(&ws_out, &ws_state) => { },
  }

  tracing::info!("Closed.");
}

async fn ws_read(
  mut ws_in: SplitStream<WebSocket>,
  ws_out: &tokio::sync::Mutex<SplitSink<WebSocket, Message>>,
  state: Arc<AppState>,
  ws_state: &Mutex<WsState>,
) {
  loop {
    let msg = ws_in.next().await;

    let exit = handle_read(state.clone(), ws_out, ws_state, msg).await;

    if exit {
      break;
    }

    // TODO(config)
    if ws_state.lock().quick_paint > 3 {
      tracing::warn!("Closed due to quick paint");
      break;
    }

    if ws_state.lock().trash_pack > 0 {
      tracing::warn!("Closed due to trash pack");
      break;
    }
  }
}

async fn recv_paint(
  state: Arc<AppState>,
  ws_state: &Mutex<WsState>,
  ws_paints: &Mutex<Vec<Pixel>>,
) {
  let mut receiver = state.sender.subscribe();

  loop {
    let msg = receiver.recv().await;

    if msg.is_err() {
      continue;
    }

    let paint = msg.unwrap();

    {
      let ws_state = ws_state.lock();

      if !ws_state.readonly {
        ws_paints.lock().push(paint);
      }
    }
  }
}

async fn ws_write(
  ws_out: &tokio::sync::Mutex<SplitSink<WebSocket, Message>>,
  ws_paints: &Mutex<Vec<Pixel>>,
) {
  // TODO(config)
  let mut interval = tokio::time::interval(Duration::from_millis(250));

  loop {
    interval.tick().await;

    let num = ws_paints.lock().len();

    if num == 0 {
      continue;
    }

    let mut msg = Vec::with_capacity(num * 7 + 1);

    msg.push(0xfa);

    for pixel in ws_paints.lock().iter() {
      let pixel_bytes: [u8; 7] = pixel.into();
      msg.extend_from_slice(&pixel_bytes);
    }

    ws_paints.lock().clear();

    let res = ws_out.lock().await.send(Message::Binary(msg)).await;

    if res.is_err() {
      tracing::warn!("Closed due to failed to send pixels");
      break;
    }
  }
}

async fn heartbeat(
  ws_out: &tokio::sync::Mutex<SplitSink<WebSocket, Message>>,
  ws_state: &Mutex<WsState>,
) {
  // TODO(config)
  let mut heartbeat = tokio::time::interval(Duration::from_secs(20));

  loop {
    heartbeat.tick().await;

    let res = ws_out.lock().await.send(Message::Binary(vec![0xf8])).await;
    if res.is_err() {
      tracing::warn!("Closed due to failed to send `ping`");
      break;
    }

    tracing::info!("Ping!");

    // TODO(config)
    tokio::time::sleep(Duration::from_secs(10)).await;

    {
      let mut ws_state = ws_state.lock();

      if !ws_state.get_pong {
        tracing::warn!("Closed without Pong");
        break;
      }

      ws_state.get_pong = false;
    }
  }
}
