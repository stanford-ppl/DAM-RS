mod building;
mod executed;
mod initialized;
mod programdata;

#[cfg(feature = "dot")]
mod dot;

use derive_builder::Builder;
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

#[derive(Default, Debug, Clone)]
pub enum LoggingOptions {
    #[default]
    None,
    Mongodb(String),
}

#[derive(Default, Debug, Builder, Clone)]
pub struct InitializationOptions {
    pub(super) run_flavor_inference: bool,
    pub(super) logging: LoggingOptions,
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
