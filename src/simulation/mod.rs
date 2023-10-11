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

/// Options for executing an [Initialized] program.
#[derive(Builder, Default)]
#[builder(pattern = "owned")]
pub struct RunOptions {
    /// Options for how to schedule the child threads
    #[builder(setter(into), default)]
    mode: RunMode,

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
        Self::Blanket(LogFilter::default()).into()
    }
}

/// Execution mode for each thread
#[derive(Debug, Default, Clone, Copy)]
pub enum RunMode {
    /// Execute under the default OS scheduler, such as CFS for Linux
    #[default]
    Simple,

    /// Use FIFO (real-time) scheduling. This is higher performance, but may lead to starvation of other processes.
    FIFO,
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
