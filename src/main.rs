mod save;
mod channel;
mod routers;

use std::{sync::Arc, collections::HashMap};

use parking_lot::Mutex;
use chrono::{DateTime, Local};
use tokio::sync::broadcast::{self, Sender};
use sea_orm::{Database, DatabaseConnection, EntityTrait};
use axum::{Router, routing::{get, post}};

use tracing_subscriber::{prelude::*, filter};

use yur_paintboard::entities::{prelude::*, board, paint};
use crate::{save::{save_board, save_actions}, channel::ChannelMsg};

pub struct AppState {
  db: DatabaseConnection,
  sender: Sender<ChannelMsg>,
  board: HashMap<(u16, u16), Mutex<board::Model>>,
  user_paint: Mutex<HashMap<i32, DateTime<Local>>>,
  actions: Mutex<Vec<paint::ActiveModel>>,
}

#[tokio::main]
async fn main() {
  let target_layer = filter::Targets::new()
    .with_target("sqlx", tracing::Level::ERROR)
    .with_target("yur_paintboard", tracing::Level::INFO);

  let fmt_layer = tracing_subscriber::fmt::layer()
    .with_target(false);

  tracing_subscriber::registry()
    .with(target_layer)
    .with(filter::LevelFilter::INFO)
    .with(fmt_layer)
    .init();

  let db = Database::connect("sqlite:./data.db?mode=rwc").await
    .expect("Error opening database!");

  let board = Board::find()
    .all(&db).await
    .expect("Error fetching board!");

  // TODO(config)
  let (sender, _) = broadcast::channel::<ChannelMsg>(65536);

  let mut now_board = HashMap::new();
  let mut old_board = HashMap::new();

  for pixel in board {
    let pos = (pixel.x as u16, pixel.y as u16);
    now_board.insert(pos, Mutex::new(pixel.clone()));
    old_board.insert(pos, pixel);
  }

  let init_state = AppState {
    db,
    sender,
    board: now_board,
    user_paint: Mutex::new(HashMap::new()),
    actions: Mutex::new(vec![]),
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

  let save_board_task = save_board(shared_state.clone(), old_board);
  let save_actions_task = save_actions(shared_state);

  tracing::info!("Listening on 127.0.0.1:2895...");

  let (res, _, _) =
    futures::future::join3(
      web_task,
      save_board_task,
      save_actions_task,
    ).await;

  res.unwrap();
}
