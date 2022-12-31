use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Paint {
  pub x: i32,
  pub y: i32,
  pub c: (u8, u8, u8),
}

pub fn color_to_hex(color: (u8, u8, u8)) -> String {
  format!("#{:02X}{:02X}{:02X}", color.0, color.1, color.2)
}

pub fn hex_to_bin(hex: &str) -> [u8; 3] {
  let r = u8::from_str_radix(&hex[1..3], 16).unwrap();
  let g = u8::from_str_radix(&hex[3..5], 16).unwrap();
  let b = u8::from_str_radix(&hex[5..7], 16).unwrap();

  [r, g, b]
}
