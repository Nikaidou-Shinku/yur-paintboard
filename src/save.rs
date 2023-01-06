use std::{sync::Arc, collections::HashMap};

use sea_orm::{ActiveValue, EntityTrait, sea_query::OnConflict};

use yur_paintboard::entities::{prelude::*, board, paint};
use crate::AppState;

pub async fn save_board(
  state: Arc<AppState>,
  mut old_board: HashMap<(u16, u16), board::Model>,
) {
  loop {
    // TODO(config)
    // 5 minutes
    tokio::time::sleep(std::time::Duration::from_secs(300)).await;

    println!("[BD] Start saving board...");

    let mut tasks = vec![];

    for pixel in &state.board {
      let old_pixel = old_board.get(pixel.0).unwrap();
      let now_pixel = pixel.1.lock().unwrap();

      if old_pixel.time != now_pixel.time { // is it accurate enough?
        tasks.push(board::ActiveModel {
          x: ActiveValue::set(now_pixel.x),
          y: ActiveValue::set(now_pixel.y),
          color: ActiveValue::set(now_pixel.color.clone()),
          uid: ActiveValue::set(now_pixel.uid),
          time: ActiveValue::set(now_pixel.time),
        });

        old_board.insert(pixel.0.to_owned(), now_pixel.to_owned());
      }
    }

    println!("[BD] Diff board size: {}", tasks.len());

    // TODO(config)
    let tasks = tasks.chunks(600) // pack 600 pixels per task
      .map(|chunk| chunk.to_owned())
      .collect::<Vec<Vec<board::ActiveModel>>>();

    for task in tasks {
      let res = Board::insert_many(task)
        .on_conflict(
          OnConflict::columns([board::Column::X, board::Column::Y])
            .update_columns([
              board::Column::Color,
              board::Column::Uid,
              board::Column::Time,
            ])
            .to_owned()
        )
        .exec(&state.db).await;

      if res.is_err() {
        eprintln!("[BD] Save board failed!");
      }
    }

    println!("[BD] Save board success!");
  }
}

pub async fn save_actions(
  state: Arc<AppState>,
) {
  loop {
    // TODO(config)
    // 8 minutes
    tokio::time::sleep(std::time::Duration::from_secs(480)).await;

    println!("[AT] Start saving actions...");

    let actions = {
      let mut actions = state.actions
        .lock().unwrap();

      let res = actions.clone();

      actions.clear();

      res
    };

    println!("[AT] Actions num: {}", actions.len());

    // TODO(config)
    let tasks = actions.chunks(600) // pack 600 actions per task
      .map(|chunk| chunk.to_owned())
      .collect::<Vec<Vec<paint::ActiveModel>>>();

    for task in tasks {
      let res = Paint::insert_many(task)
        .exec(&state.db).await;

      if res.is_err() {
        eprintln!("[AT] Save actions failed!");
      }
    }


    println!("[AT] Save actions success!");
  }
}
