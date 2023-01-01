use uuid::Uuid;

use crate::paint::Paint;

#[derive(Clone, Debug)]
pub enum ChannelMsg {
  Close(Uuid),
  Paint(Paint),
}
