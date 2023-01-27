use std::collections::HashMap;

use chrono::Local;
use sea_orm::{Database, DatabaseConnection, EntityTrait};

use yur_paintboard::entities::{board, prelude::*};

async fn board(db: &DatabaseConnection) {
  let board = Board::find().all(db).await.expect("Error fetching board!");

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
  println!();
}

async fn actions(db: &DatabaseConnection) {
  let actions = Paint::find().all(db).await.expect("Error fetching paint!");

  let mut paint_info = HashMap::new();
  let mut pixel_num = 0;

  for action in actions.iter() {
    let uid = action.uid;

    paint_info
      .entry(uid)
      .and_modify(|num| *num += 1)
      .or_insert(1);

    pixel_num += 1;
  }

  let mut paint_info: Vec<_> = paint_info.iter().collect();
  paint_info.sort_by(|a, b| b.1.cmp(a.1));

  println!("Actions ({} users, {pixel_num} pixels):", paint_info.len());
  for user in paint_info {
    println!("UID: {:6}, Number of pixels: {:6}", user.0, user.1);
  }
}

#[tokio::main]
async fn main() {
  let db = Database::connect("sqlite:./data.db?mode=rwc")
    .await
    .expect("Error opening database!");

  board(&db).await;
  actions(&db).await;
}
