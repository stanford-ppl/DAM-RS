use serde::{Deserialize, Serialize};

use super::ChannelID;

#[derive(Serialize, Deserialize, Debug)]
pub enum SendEvent {
    Send(ChannelID),
    Len(ChannelID, usize),
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum ReceiverEvent {
    Peek(ChannelID),
    Recv(ChannelID),
}
