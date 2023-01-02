use uuid::Uuid;

use yur_paintboard::pixel::Pixel;

#[derive(Clone, Debug)]
pub enum ChannelMsg {
  Close(Uuid),
  Paint(Pixel),
}
