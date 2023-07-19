use dam_core::identifier::Identifier;
use serde::{Deserialize, Serialize};

use super::ChannelID;

#[derive(Serialize, Deserialize, Debug)]
pub enum SendEvent {
    TrySend(ChannelID),
    EnqueueStart(ChannelID),
    EnqueueFinish(ChannelID),
    AttachSender(ChannelID, Identifier),
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum ReceiverEvent {
    Peek(ChannelID),
    PeekNextStart(ChannelID),
    PeekNextFinish(ChannelID),
    DequeueStart(ChannelID),
    DequeueFinish(ChannelID),
    AttachReceiver(ChannelID, Identifier),
}
