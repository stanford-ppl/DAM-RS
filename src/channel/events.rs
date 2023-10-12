use dam_macros::event_type_internal;
use serde::{Deserialize, Serialize};

use crate::datastructures::Identifier;

use super::ChannelID;

#[derive(Serialize, Deserialize, Debug)]
#[event_type_internal]
pub enum SendEvent {
    TrySend(ChannelID),
    EnqueueStart(ChannelID),
    EnqueueFinish(ChannelID),
    AttachSender(ChannelID, Identifier),
    Cleanup(ChannelID),
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
#[event_type_internal]
pub enum ReceiverEvent {
    Peek(ChannelID),
    PeekNextStart(ChannelID),
    PeekNextFinish(ChannelID),
    DequeueStart(ChannelID),
    DequeueFinish(ChannelID),
    AttachReceiver(ChannelID, Identifier),
    Cleanup(ChannelID),
}
