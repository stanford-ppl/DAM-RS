use dam_core::prelude::*;
use dam_macros::event_type;
use serde::{Deserialize, Serialize};

use super::ChannelID;

#[derive(Serialize, Deserialize, Debug)]
#[event_type]
pub enum SendEvent {
    TrySend(ChannelID),
    EnqueueStart(ChannelID),
    EnqueueFinish(ChannelID),
    AttachSender(ChannelID, Identifier),
    Cleanup(ChannelID),
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[event_type]
pub enum ReceiverEvent {
    Peek(ChannelID),
    PeekNextStart(ChannelID),
    PeekNextFinish(ChannelID),
    DequeueStart(ChannelID),
    DequeueFinish(ChannelID),
    AttachReceiver(ChannelID, Identifier),
    Cleanup(ChannelID),
}
