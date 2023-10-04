mod building;
mod executed;
mod initialized;
mod programdata;

#[cfg(feature = "dot")]
mod dot;

#[cfg(feature = "dot")]
pub use dot::DotConvertible;

// Export all of the program states
pub use building::ProgramBuilder;
pub use executed::Executed;
pub use initialized::Initialized;

use crate::channel::ChannelID;
use dam_core::prelude::*;
use thiserror::Error;

#[derive(Debug, Default, Clone, Copy)]
pub enum RunMode {
    #[default]
    Simple,
    FIFO,
}

#[derive(Default)]
pub struct InitializationOptions {
    pub run_flavor_inference: bool,
}

#[derive(Error, Debug)]
pub enum InitializationError {
    #[error("Disconnected Sender on channel: {0:?}")]
    DisconnectedSender(ChannelID),

    #[error("Disconnected Receiver on channel: {0:?}")]
    DisconnectedReceiver(ChannelID),

    #[error("Unregistered Node: {0}")]
    UnregisteredNode(Identifier),
}
