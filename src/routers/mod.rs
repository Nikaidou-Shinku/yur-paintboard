mod auth;
mod ws;

pub use auth::{auth, verify};
pub use ws::ws;

use serde::Serialize;

#[derive(Serialize)]
pub struct Resp<T> {
  code: i32,
  data: T,
}

impl<T> From<T> for Resp<T> {
  fn from(data: T) -> Self {
    Resp { code: 0, data }
  }
}

#[derive(Serialize)]
pub struct ErrResp {
  msg: String,
}

impl From<&str> for ErrResp {
  fn from(msg: &str) -> Self {
    ErrResp { msg: msg.to_owned() }
  }
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum ErrOr<T> {
  Err(ErrResp),
  Ok(Resp<T>),
}
