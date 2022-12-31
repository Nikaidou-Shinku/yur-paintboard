use crate::paint::Paint;

#[derive(Clone, Debug)]
pub enum ChannelMsg {
  Close,
  Paint(Paint),
}
