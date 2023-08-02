use dam_core::identifier::Identifier;
use serde::{Deserialize, Serialize};

use super::ChannelID;

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum ChannelEvent {
    SendEvent(SendEvent),
    ReceiverEvent(ReceiverEvent),
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum SendEvent {
    TrySend(ChannelID),
    EnqueueStart(ChannelID),
    EnqueueFinish(ChannelID),
    AttachSender(ChannelID, Identifier),
    Cleanup(ChannelID),
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum ReceiverEvent {
    Peek(ChannelID),
    PeekNextStart(ChannelID),
    PeekNextFinish(ChannelID),
    DequeueStart(ChannelID),
    DequeueFinish(ChannelID),
    AttachReceiver(ChannelID, Identifier),
    Cleanup(ChannelID),
}
