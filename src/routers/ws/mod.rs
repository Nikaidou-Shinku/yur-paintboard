mod read;

use std::{sync::Arc, time::Duration};

use axum::{
  extract::{
    State,
    WebSocketUpgrade,
    ws::{WebSocket, Message},
  },
  response::Response,
};

use crate::AppState;
use read::ws_read;

pub async fn ws(
  State(state): State<Arc<AppState>>,
  ws: WebSocketUpgrade,
) -> Response {
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
async fn handle_ws(
  state: Arc<AppState>,
  mut socket: WebSocket,
) {
  let mut receiver = state.sender.subscribe();
  let mut ws_paints = vec![];
  // TODO(config)
  let mut interval = tokio::time::interval(Duration::from_millis(250));
  let mut heartbeat = tokio::time::interval(Duration::from_secs(20));

  let mut ws_state = WsState {
    uid: None,
    readonly: true,
    get_pong: false,
    quick_paint: 0,
    trash_pack: 0,
  };

  loop {
    tokio::select! {
      msg = socket.recv() => {
        let end = ws_read(state.clone(), &mut socket, &mut ws_state, msg).await;
        if end {
          break;
        }
      },
      msg = receiver.recv() => {
        if let Ok(paint) = msg {
          if !ws_state.readonly {
            ws_paints.push(paint);
          }
        }
      },
      _ = interval.tick() => {
        if ws_paints.len() > 0 {
          let mut msg = Vec::with_capacity(ws_paints.len() * 7 + 1);
  
          msg.push(0xfa);

          for pixel in ws_paints.iter() {
            let pixel_bytes: [u8; 7] = pixel.into();
            msg.extend_from_slice(&pixel_bytes);
          }

          ws_paints.clear();

          let res = socket.send(Message::Binary(msg)).await;
  
          if res.is_err() {
            break;
          }
        }
      },
      _ = heartbeat.tick() => {
        let res = socket.send(Message::Binary(vec![0xf8])).await;
        if res.is_err() {
          break;
        }

        tokio::time::sleep(Duration::from_secs(10)).await;

        if !ws_state.get_pong {
          break;
        }

        ws_state.get_pong = false;
      },
    }
  }

  tracing::info!("Closed.");
}
