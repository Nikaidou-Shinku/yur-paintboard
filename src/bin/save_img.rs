use sea_orm::{Database, EntityTrait};
use clap::Parser;

use yur_paintboard::{
  consts::{WIDTH, HEIGHT},
  entities::prelude::*,
  pixel::hex_to_bin,
};

#[derive(Parser)]
#[command(name = "save_img")]
#[command(author = "yurzhang")]
#[command(about = "Save the paintboard to an image file.")]
#[command(version, long_about = None)]
struct Args {
  #[arg(short, long, default_value_t = String::from("result.png"))]
  output: String,
}

#[tokio::main]
async fn main() {
  let args = Args::parse();

  let db = Database::connect("sqlite:./data.db?mode=rwc").await
    .expect("Error opening database!");

  let board = Board::find()
    .all(&db).await
    .expect("Error fetching board!");

  let width = WIDTH.into();
  let height = HEIGHT.into();

  let mut imgbuf = image::ImageBuffer::new(width, height);

  for item in board {
    let x = item.x as u32;
    let y = item.y as u32;
    let color = hex_to_bin(&item.color);

    let pixel = imgbuf.get_pixel_mut(x, y);
    *pixel = image::Rgb(color);
  }

  imgbuf.save(args.output).unwrap();
}
