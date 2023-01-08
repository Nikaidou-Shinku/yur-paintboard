use std::sync::Arc;

use bytes::{BytesMut, BufMut, Buf, Bytes};
use uuid::Uuid;
use futures::{stream::SplitSink, SinkExt};
use tokio::sync::Mutex;
use axum::extract::ws::{Message, WebSocket};

use yur_paintboard::{consts::{WIDTH, HEIGHT}, pixel::hex_to_bytes};
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
          ws_paints.lock().await.push(paint);
        },
        ChannelMsg::Board(session) => {
          if session == ws_session {
            let board = get_board(state.clone());
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

      {
        let mut ws_paints = ws_paints.lock().await;

        if ws_paints.len() > 0 {
          let mut msg = BytesMut::with_capacity(ws_paints.len() * 7 + 1);

          msg.put_u8(0xfa);

          for pixel in ws_paints.iter() {
            msg.put::<Bytes>(pixel.into());
          }

          let res = ws_out.lock().await
            .send(Message::Binary(msg.into())).await;

          if res.is_err() {
            *ws_over.lock().await = true;
            break;
          }

          ws_paints.clear();
        }
      }

      // TODO(config)
      tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
  };

  futures::future::join(recv_task, send_task).await;

  tracing::info!("Closed.");
}

// TODO: maybe more elegant way to do this
fn get_board(
  state: Arc<AppState>,
) -> Vec<u8> {
  let max_len = (WIDTH * HEIGHT * 3).into();
  let mut board = BytesMut::with_capacity(max_len);

  for x in 0..WIDTH {
    for y in 0..HEIGHT {
      let pixel = state.board
        .get(&(x, y)).unwrap()
        .lock();
      board.put(hex_to_bytes(&pixel.color));
    }
  }

  // TODO(config): compress level
  let raw_board = zstd::encode_all(board.reader(), 0).unwrap();
  let mut board = vec![0xfb];
  board.extend_from_slice(&raw_board);

  board
}
