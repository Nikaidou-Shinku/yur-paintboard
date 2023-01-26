use chrono::Local;
use clap::Parser;
use sea_orm::{sea_query::OnConflict, ActiveValue, Database, EntityTrait};

use yur_paintboard::{
  consts::{HEIGHT, WIDTH},
  entities::{board, prelude::*},
};

#[derive(Parser)]
#[command(name = "setup")]
#[command(author = "yurzhang")]
#[command(about = "Setup the paintboard.")]
#[command(version, long_about = None)]
struct Args {
  #[clap(short, long, default_value_t = String::from("#ffffff"))]
  color: String,
}

fn check_color(color: &str) -> bool {
  if color.len() != 7 {
    return false;
  }

  if !color.starts_with('#') {
    return false;
  }

  for c in color.chars().skip(1) {
    if !c.is_ascii_hexdigit() {
      return false;
    }
  }

  true
}

#[tokio::main]
async fn main() {
  let args = Args::parse();

  if !check_color(&args.color) {
    eprintln!("Invalid color: {}", args.color);
    std::process::exit(1);
  }

  let db = Database::connect("sqlite:./data.db?mode=rwc")
    .await
    .expect("Error opening database!");

  let now = Local::now();

  for x in 0..WIDTH {
    let tasks = (0..HEIGHT).map(|y| board::ActiveModel {
      x: ActiveValue::set(x.into()),
      y: ActiveValue::set(y.into()),
      color: ActiveValue::set(args.color.clone()),
      uid: ActiveValue::set(-1),
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
          .to_owned(),
      )
      .exec(&db)
      .await;

    if let Err(err) = res {
      eprintln!("[C{x}] Error inserting pixel: {err}");
    }
  }
}
