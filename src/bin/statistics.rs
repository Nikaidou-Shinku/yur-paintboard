use std::collections::HashMap;

use chrono::Local;
use sea_orm::{Database, EntityTrait};

use yur_paintboard::entities::{prelude::*, board};

#[tokio::main]
async fn main() {
  let db = Database::connect("sqlite:./data.db?mode=rwc").await
    .expect("Error opening database!");

  let board = Board::find()
    .all(&db).await
    .expect("Error fetching board!");

  let mut earliest_paint = board::Model {
    x: -1,
    y: -1,
    color: "#ffffff".to_owned(),
    uid: -1,
    time: Local::now(),
  };

  let mut board_info = HashMap::new();

  for pixel in board {
    let uid = pixel.uid;

    if pixel.time < earliest_paint.time {
      earliest_paint = pixel;
    }

    board_info
      .entry(uid)
      .and_modify(|num| *num += 1)
      .or_insert(1);
  }

  let mut board_info: Vec<_> = board_info.iter().collect();
  board_info.sort_by(|a, b| b.1.cmp(a.1));

  println!("The earliest pixel:\n{:?}", earliest_paint);

  println!("Ranking:");
  for item in board_info {
    println!("UID: {}, Number of pixels: {}", item.0, item.1);
  }
}
