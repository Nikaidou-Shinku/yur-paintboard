mod save;
mod channel;
mod routers;

use std::{sync::{Arc, Mutex}, collections::HashMap};

use uuid::Uuid;
use chrono::{DateTime, Local};
use tokio::sync::broadcast::{self, Sender};
use sea_orm::{Database, DatabaseConnection, EntityTrait};
use axum::{Router, routing::{get, post}};

use crate::{save::save_board, channel::ChannelMsg};
use yur_paintboard::entities::{prelude::*, board};

pub struct AppState {
  db: DatabaseConnection,
  sender: Sender<ChannelMsg>,
  board: Vec<Mutex<board::Model>>,
  user_ws: Mutex<HashMap<i32, Option<Uuid>>>,
  user_paint: Mutex<HashMap<i32, DateTime<Local>>>,
}

#[tokio::main]
async fn main() {
  let db = Database::connect("sqlite:./data.db?mode=rwc").await
    .expect("Error opening database!");

  let board = Board::find()
    .all(&db).await
    .expect("Error fetching board!");

  // TODO(config)
  let (sender, _) = broadcast::channel::<ChannelMsg>(65536);

  let now_board = board.iter()
    .map(|pixel| Mutex::new(pixel.clone()))
    .collect();

  let init_state = AppState {
    db,
    sender,
    board: now_board,
    user_ws: Mutex::new(HashMap::new()),
    user_paint: Mutex::new(HashMap::new()),
  };
  let shared_state = Arc::new(init_state);

  let app = Router::new()
    .route("/", get(|| async { "Just paint freely!" }))
    .route("/auth", post(routers::auth))
    .route("/verify", post(routers::verify))
    .route("/ws", get(routers::ws))
    .with_state(shared_state.clone());

  // TODO(config)
  let web_task = axum::Server::bind(&"127.0.0.1:2895".parse().unwrap())
    .serve(app.into_make_service());

  let save_task = save_board(shared_state, board);

  println!("[MN] Listening on 127.0.0.1:2895...");

  let (res, _) = futures::future::join(web_task, save_task).await;

  res.unwrap();
}
