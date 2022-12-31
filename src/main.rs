mod channel;
mod routers;
mod paint;

use std::sync::Arc;

use tokio::sync::broadcast::{self, Sender};
use sea_orm::{Database, DatabaseConnection};
use axum::{Router, routing::{get, post}};

use channel::ChannelMsg;

pub struct AppState {
  db: DatabaseConnection,
  sender: Sender<ChannelMsg>,
}

#[tokio::main]
async fn main() {
  let db = Database::connect("sqlite:./data.db?mode=rwc").await
    .expect("Error opening database!");

  let (sender, _) = broadcast::channel::<ChannelMsg>(256);

  let shared_state = Arc::new(AppState { db, sender });

  let app = Router::new()
    .route("/", get(|| async { "Just paint freely!" }))
    .route("/auth", post(routers::auth))
    .route("/verify", post(routers::verify))
    .route("/ws", get(routers::ws))
    .with_state(shared_state);

  axum::Server::bind(&"0.0.0.0:2895".parse().unwrap())
    .serve(app.into_make_service()).await.unwrap();
}
