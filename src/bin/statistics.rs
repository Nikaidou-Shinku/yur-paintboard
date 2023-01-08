use std::collections::HashMap;

use chrono::Local;
use sea_orm::{Database, EntityTrait, DatabaseConnection};

use yur_paintboard::entities::{prelude::*, board};

async fn board(db: &DatabaseConnection) {
  let board = Board::find()
    .all(db).await
    .expect("Error fetching board!");

  let mut earliest_paint = board::Model {
    x: -1,
    y: -1,
    color: "#ffffff".to_owned(),
    uid: -1,
    time: Local::now(),
  };

  let mut board_info = HashMap::new();
  let mut pixel_num = 0;

  for pixel in board {
    let uid = pixel.uid;

    if uid == -1 {
      continue;
    }

    if pixel.time < earliest_paint.time {
      earliest_paint = pixel;
    }

    board_info
      .entry(uid)
      .and_modify(|num| *num += 1)
      .or_insert(1);

    pixel_num += 1;
  }

  let mut board_info: Vec<_> = board_info.iter().collect();
  board_info.sort_by(|a, b| b.1.cmp(a.1));

  println!("The earliest pixel:\n{earliest_paint:?}\n");

  println!("Ranking ({} users, {pixel_num} pixels):", board_info.len());
  for item in board_info {
    println!("UID: {:6}, Number of pixels: {:6}", item.0, item.1);
  }
}

async fn users(db: &DatabaseConnection) {
  let auth_users = Auth::find()
    .all(db).await
    .expect("Error fetching auth!");

  let sessions = Session::find()
    .all(db).await
    .expect("Error fetching session!");

  println!(
    "Auth users: {}, Sessions: {}",
    auth_users.len(),
    sessions.len(),
  );
}

async fn actions(db: &DatabaseConnection) {
  let actions = Paint::find()
    .all(db).await
    .expect("Error fetching paint!");

  println!("Actions: {}\n", actions.len());
}

#[tokio::main]
async fn main() {
  let db = Database::connect("sqlite:./data.db?mode=rwc").await
    .expect("Error opening database!");

  users(&db).await;
  actions(&db).await;
  board(&db).await;
}
