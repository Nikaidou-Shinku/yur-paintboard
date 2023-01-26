mod save;
mod ws;

use std::{collections::HashMap, sync::Arc};

use axum::{routing::get, Router};
use chrono::{DateTime, Local};
use jsonwebtoken::DecodingKey;
use parking_lot::Mutex;
use sea_orm::{Database, DatabaseConnection, EntityTrait};
use tokio::sync::broadcast::{self, Sender};

use tracing_subscriber::{filter, prelude::*};

use crate::save::{save_actions, save_board};
use yur_paintboard::{
  entities::{board, paint, prelude::*},
  pixel::Pixel,
};

pub struct AppState {
  pubkey: DecodingKey,
  db: DatabaseConnection,
  sender: Sender<Pixel>,
  board: HashMap<(u16, u16), Mutex<board::Model>>,
  user_paint: Mutex<HashMap<i32, DateTime<Local>>>,
  actions: Mutex<Vec<paint::ActiveModel>>,
}

#[tokio::main]
async fn main() {
  let target_layer = filter::Targets::new()
    .with_target("sqlx", tracing::Level::ERROR)
    .with_target("yur_paintboard", tracing::Level::INFO);

  let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);

  tracing_subscriber::registry()
    .with(target_layer)
    .with(filter::LevelFilter::INFO)
    .with(fmt_layer)
    .init();

  // TODO(config)
  let pubkey = reqwest::get("https://sso.yurzhang.com/pubkey")
    .await
    .expect("Error fetching public key")
    .bytes()
    .await
    .expect("Error decode public key");

  let pubkey = DecodingKey::from_ed_pem(&pubkey).expect("Error loading public key");

  let db = Database::connect("sqlite:./data.db?mode=rwc")
    .await
    .expect("Error opening database!");

  let board = Board::find().all(&db).await.expect("Error fetching board!");

  // TODO(config)
  let (sender, _) = broadcast::channel::<Pixel>(65536);

  let mut now_board = HashMap::new();
  let mut old_board = HashMap::new();

  for pixel in board {
    let pos = (pixel.x as u16, pixel.y as u16);
    now_board.insert(pos, Mutex::new(pixel.clone()));
    old_board.insert(pos, pixel);
  }

  let init_state = AppState {
    pubkey,
    db,
    sender,
    board: now_board,
    user_paint: Mutex::new(HashMap::new()),
    actions: Mutex::new(vec![]),
  };
  let shared_state = Arc::new(init_state);

  let app = Router::new()
    .route("/", get(|| async { "Just paint freely!" }))
    .route("/ws", get(ws::ws))
    .with_state(shared_state.clone());

  // TODO(config)
  let web_task =
    axum::Server::bind(&"127.0.0.1:2895".parse().unwrap()).serve(app.into_make_service());

  let save_board_task = save_board(shared_state.clone(), old_board);
  let save_actions_task = save_actions(shared_state);

  tracing::info!("Listening on 127.0.0.1:2895...");

  let (res, _, _) = futures::future::join3(web_task, save_board_task, save_actions_task).await;

  res.unwrap();
}
