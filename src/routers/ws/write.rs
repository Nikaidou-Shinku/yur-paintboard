use std::sync::Arc;

use uuid::Uuid;
use futures::{stream::SplitSink, SinkExt};
use tokio::sync::Mutex;
use axum::extract::ws::{Message, WebSocket};

use yur_paintboard::{consts::{WIDTH, HEIGHT}, pixel::hex_to_bin};
use crate::{AppState, channel::ChannelMsg};

#[tracing::instrument(name = "write", skip_all)]
pub async fn ws_write(
  state: Arc<AppState>,
  ws_session: Uuid,
  ws_out: SplitSink<WebSocket, Message>,
) {
  let ws_over = Mutex::new(false);
  let ws_paints = Mutex::new(vec![]);
  let ws_out = Mutex::new(ws_out);
  let mut ws_readonly = true;

  let recv_task = async {
    let mut receiver = state.sender.subscribe();

    loop {
      if *ws_over.lock().await {
        break;
      }

      let msg = receiver.recv().await;

      if msg.is_err() {
        continue;
      }

      let msg = msg.unwrap();

      match msg {
        ChannelMsg::Close(session) => {
          if session == ws_session {
            *ws_over.lock().await = true;
            break;
          }
        },
        ChannelMsg::Paint(paint) => {
          if !ws_readonly {
            ws_paints.lock().await.push(paint);
          }
        },
        ChannelMsg::Board(session) => {
          if session == ws_session {
            let board = get_board(state.clone());

            ws_readonly = false;

            let res = ws_out.lock().await
              .send(Message::Binary(board)).await;

            if res.is_err() {
              *ws_over.lock().await = true;
              break;
            }

            tracing::info!("Sent board.");
          }
        }
      }
    }
  };

  let send_task = async {
    loop {
      if *ws_over.lock().await {
        break;
      }

      let msg = {
        let mut ws_paints = ws_paints.lock().await;

        if ws_paints.len() > 0 {
          let mut msg = Vec::with_capacity(ws_paints.len() * 7 + 1);

          msg.push(0xfa);

          for pixel in ws_paints.iter() {
            let pixel_bytes: [u8; 7] = pixel.into();
            msg.extend_from_slice(&pixel_bytes);
          }

          ws_paints.clear();

          Some(msg)
        } else {
          None
        }
      };

      if let Some(msg) = msg {
        let res = ws_out.lock().await
          .send(Message::Binary(msg)).await;

        if res.is_err() {
          *ws_over.lock().await = true;
          break;
        }
      }

      // TODO(config)
      tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
  };

  futures::future::join(recv_task, send_task).await;

  tracing::trace!("Closed.");
}

// TODO: maybe more elegant way to do this
fn get_board(
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
