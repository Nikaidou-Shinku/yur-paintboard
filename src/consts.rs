use chrono::NaiveTime;
use lazy_static::lazy_static;

// TODO(config)
pub const WIDTH: u16 = 1000;
pub const HEIGHT: u16 = 600;

lazy_static! {
  pub static ref BEGIN_TIME: NaiveTime = NaiveTime::from_hms_opt(15, 0, 0).unwrap();
  pub static ref END_TIME: NaiveTime = NaiveTime::from_hms_opt(20, 0, 0).unwrap();
}
