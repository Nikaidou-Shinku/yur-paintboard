use std::collections::HashMap;

use chrono::{Local, TimeZone};
use sea_orm::{Database, EntityTrait};

use yur_paintboard::{
  consts::{WIDTH, HEIGHT},
  entities::prelude::*,
  pixel::hex_to_bin,
};

#[tokio::main]
async fn main() {
  let db = Database::connect("sqlite:./data.db?mode=rwc").await
    .expect("Error opening database!");

  let actions = Paint::find()
    .all(&db).await
    .expect("Error fetching actions!");

  let mut board = HashMap::new();

  for x in 0..WIDTH {
    for y in 0..HEIGHT {
      board.insert((x, y), [240, 240, 240]);
    }
  }

  let mut begin_time = Local.with_ymd_and_hms(2023, 1, 14, 14, 0, 0).unwrap();
  let end_time = Local.with_ymd_and_hms(2023, 1, 16, 14, 0, 0).unwrap();

  let mut action_idx = 0;
  let mut pic_idx = 1;

  std::fs::create_dir_all("./frames").unwrap();

  while begin_time <= end_time {
    while action_idx < actions.len() && actions[action_idx].time < begin_time {
      let action = &actions[action_idx];
      let pos = (action.x as u16, action.y as u16);
      board.insert(pos, hex_to_bin(&action.color));
      action_idx += 1;
    }

    let mut imgbuf = image::ImageBuffer::new(WIDTH.into(), HEIGHT.into());

    for item in board.iter() {
      let (x, y) = *item.0;
      let pixel = imgbuf.get_pixel_mut(x.into(), y.into());
      *pixel = image::Rgb(*item.1);
    }

    imgbuf.save(format!("./frames/{pic_idx}.png")).unwrap();

    begin_time += chrono::Duration::seconds(20);
    pic_idx += 1;
  }
}
