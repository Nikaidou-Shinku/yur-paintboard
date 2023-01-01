use std::sync::Arc;

use sea_orm::{ActiveValue, EntityTrait, sea_query::OnConflict};

use yur_paintboard::entities::{prelude::*, board};
use crate::AppState;

pub async fn save_board(state: Arc<AppState>) {
  loop {
    // 5 minutes
    tokio::time::sleep(std::time::Duration::from_secs(300)).await;

    println!("[BD] Start saving board...");

    let tasks = {
      let old_board = state.last_board.lock().unwrap();

      state.board
        .lock().unwrap()
        .iter().enumerate()
        .filter(|(idx, pixel)| &&old_board[*idx] != pixel)
        .map(|(_, pixel)| board::ActiveModel {
          x: ActiveValue::set(pixel.x),
          y: ActiveValue::set(pixel.y),
          color: ActiveValue::set(pixel.color.clone()),
          uid: ActiveValue::set(pixel.uid),
          time: ActiveValue::set(pixel.time),
        })
        .collect::<Vec<board::ActiveModel>>()
    };

    println!("[BD] Diff board size: {}", tasks.len());

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
        eprintln!("[BD] Update board failed!");
      }
    }

    *state.last_board.lock().unwrap() = state.board.lock().unwrap().clone();

    println!("[BD] Save board success!");
  }
}
