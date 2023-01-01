use chrono::Local;
use sea_orm::{Database, ActiveValue, EntityTrait, sea_query::OnConflict};

use yur_paintboard::{consts::{WIDTH, HEIGHT}, entities::{prelude::*, board}};

#[tokio::main]
async fn main() {
  let db = Database::connect("sqlite:./data.db?mode=rwc").await
    .expect("Error opening database!");

  let now = Local::now();

  for x in 0..WIDTH {
    let tasks = (0..HEIGHT)
      .map(|y| board::ActiveModel {
        x: ActiveValue::set(x.into()),
        y: ActiveValue::set(y.into()),
        color: ActiveValue::set("#ffffff".to_owned()),
        uid: ActiveValue::set(126486),
        time: ActiveValue::set(now),
      });

    let res = Board::insert_many(tasks)
      .on_conflict(
        OnConflict::columns([board::Column::X, board::Column::Y])
          .update_columns([
            board::Column::Color,
            board::Column::Uid,
            board::Column::Time,
          ])
          .to_owned()
      )
      .exec(&db).await;

    if let Err(err) = res {
      eprintln!("[L{x}] Error inserting pixel: {err}");
    }
  }

  println!("OK!");
}
