#[derive(Clone, Debug)]
pub struct Pixel {
  pub x: u16,
  pub y: u16,
  pub color: (u8, u8, u8),
}

impl From<&Pixel> for [u8; 7] {
  fn from(pixel: &Pixel) -> Self {
    let mut res = [0; 7];

    res[0..2].copy_from_slice(&pixel.x.to_le_bytes());
    res[2..4].copy_from_slice(&pixel.y.to_le_bytes());

    let c = pixel.color;
    res[4..7].copy_from_slice(&[c.0, c.1, c.2]);

    res
  }
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
