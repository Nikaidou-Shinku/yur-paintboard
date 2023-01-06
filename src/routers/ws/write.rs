use std::sync::Arc;

use uuid::Uuid;
use futures::{stream::SplitSink, SinkExt};
use tokio::sync::Mutex;
use axum::extract::ws::{Message, WebSocket};

use crate::{AppState, channel::ChannelMsg};

#[tracing::instrument(name = "write", skip_all)]
pub async fn ws_write(
  state: Arc<AppState>,
  ws_session: Uuid,
  mut ws_out: SplitSink<WebSocket, Message>,
) {
  let ws_over = Mutex::new(false);
  let ws_paints = Mutex::new(vec![]);

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
          let mut msg = vec![0xfa];

          for pixel in ws_paints.iter() {
            let pixel: [u8; 7] = pixel.into();
            msg.extend_from_slice(&pixel);
          }

          let res = ws_out.send(Message::Binary(msg)).await;

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
