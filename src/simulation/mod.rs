mod building;
mod executed;
mod initialized;
mod programdata;

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

// A Program consists of all of its nodes and all of its edges.

trait ProgramHelper {
    #[cfg(feature = "dot")]
    fn context_id_to_name(id: Identifier) -> String {
        format!("Node_{}", id.id)
    }
}

pub trait ProgramState {
    #[cfg(feature = "dot")]
    fn to_dot(&self) -> graphviz_rust::dot_structures::Graph;
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
