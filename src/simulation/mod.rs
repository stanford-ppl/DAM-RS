//! This module defines the core of how a DAM simulation is actually executed.
//! First, construct and populate a [ProgramBuilder], which can then be validated and initialized via [ProgramBuilder::initialize]
//! The initialized graph can then be executed, returning a [Executed] object, which is a summary of the execution.
//! Programs are run-once, so re-running a program requires starting from scratch.

mod building;
mod executed;
mod initialized;
mod programdata;

mod logging_options;
pub use logging_options::*;

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
use crate::datastructures::Identifier;
use crate::logging::LogFilter;
use thiserror::Error;

pub use crate::shim::RunMode;

/// Options for executing an [Initialized] program.
#[derive(Builder, Default)]
#[builder(pattern = "owned")]
pub struct RunOptions {
    /// Options for how to schedule the child threads
    #[builder(setter(into), default)]
    mode: crate::shim::RunMode,

    /// Basic logging options
    #[builder(setter(into), default)]
    logging: LoggingOptions,

    /// Filters for which types of events to log
    #[builder(setter(into), default)]
    log_filter: LogFilterKind,
}

/// Defines what events should be logged
#[derive(Clone)]
pub enum LogFilterKind {
    /// One policy for all contexts
    Blanket(LogFilter),

    /// A per-context filter, which allows targetting spcific nodes
    PerChild(fn(Identifier) -> LogFilter),
}

impl Default for LogFilterKind {
    fn default() -> Self {
        Self::Blanket(LogFilter::default())
    }
}

/// Options for how to initialize the [ProgramBuilder] into an [Initialized] object.
#[derive(Default, Debug, Builder, Clone)]
pub struct InitializationOptions {
    /// Flavor inference (Section 6.4 of the DAM paper)
    #[builder(setter(into), default)]
    pub(super) run_flavor_inference: bool,
}

/// Various ways initializing a program can fail
#[derive(Error, Debug)]
pub enum InitializationError {
    /// All channels must have registered senders
    #[error("Disconnected Sender on channel: {0:?}")]
    DisconnectedSender(ChannelID),

    /// All non-void channels must have registered receivers
    #[error("Disconnected Receiver on channel: {0:?}")]
    DisconnectedReceiver(ChannelID),

    /// All contexts must be registered
    #[error("Unregistered Node: {0}")]
    UnregisteredNode(Identifier),
}

/// Various ways a program can fail
#[derive(Error, Debug)]
pub struct SimulationError {
    id: usize,
    underlying: anyhow::Error,
}

impl std::fmt::Display for SimulationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Simulation of {} failed with message {}",
            self.id, self.underlying
        )
    }
}
