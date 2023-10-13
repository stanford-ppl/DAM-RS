//! Logging support for DAM execution
//! Right now, the only logger we support is MongoDB, but support for SQL-type databases may be added in the future.
//! It is important to note that DAM simulations can put out hundreds of GiB to TiB of logs in a single run, so any logger must be designed for scale.

use bson::Bson;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, num::TryFromIntError};
use thiserror::Error;

// Adds a logger that does nothing.
mod null_logger;
pub use null_logger::*;

#[cfg_attr(docsrs, doc(cfg(feature = "log-mongo")))]
#[cfg(feature = "log-mongo")]
pub mod mongo_logger;

mod log_interface;
pub use log_interface::LogInterface;

mod log_functions;
pub use log_functions::*;

use crate::datastructures::Time;

use self::registry::{get_metrics_vec, METRICS};

/// Handles the registering/checking of LogEntry names
pub mod registry;

/// Errors which may occur when attempting to log.
#[derive(Error, Debug)]
pub enum LogError {
    /// Attempted to convert time (in us) to i64, but ran out of time. This is unlikely to ever happen.
    #[error("Error converting time into i64. Did we run out of time?")]
    TimeConversionError(TryFromIntError),

    /// No logprocessors were registered, so the event that was sent will never be seen.
    #[error("Could not send event! Were LogProcessors registered?")]
    SendError,

    /// The filter that was registered wasn't valid -- some of the filter types weren't registered.
    #[error(
        "Invalid Log Filter Defined: {0:?} were not registered filters! Options: {:?}",
        get_metrics_vec()
    )]
    InvalidFilter(Vec<String>),

    /// Failed to convert the message into bson.
    #[error("Serialization Error")]
    SerializationError(bson::ser::Error),
}

/// A real log entry, which is eventually serialized to some actual log.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogEntry {
    /// Time in microseconds since start of simulation
    pub(crate) timestamp: i64,

    /// Identity of the current context
    pub(crate) context: usize,

    /// Number of ticks elapsed PRIOR to this event
    pub(crate) ticks: Time,

    /// String name of the logging event type
    pub(crate) event_type: String,

    /// The actual data of the event
    pub(crate) event_data: Bson,
}

/// All logs types must expose a name, which is used by filters.
pub trait LogEvent: Serialize {
    /// The declared name of the logging type. This is used to report the the event type in the [LogEntry], as well as check filters in [LogFilter]
    const NAME: &'static str;
}

/// Log Processors are responsible for processing logs (i.e. serializing to database).
pub trait LogProcessor: Send {
    /// Starts the logging job, invoked within a dedicated thread.
    fn spawn(&mut self);
}

/// Log filtering policies
#[derive(Debug, Default, Clone)]
pub enum LogFilter {
    /// Enables ALL logging -- likely to be VERY verbose and expensive
    #[default]
    AllowAll,

    /// Only enable a subset of logs, based on their registered LogEvent::NAME
    Some(HashSet<String>),
}

impl LogFilter {
    /// Checks to see if all elements of the LogFilter are actually registered metrics.
    pub fn check(&self) -> Result<(), LogError> {
        match self {
            LogFilter::AllowAll => Ok(()),
            LogFilter::Some(set) => {
                let invalids: Vec<_> = set
                    .clone()
                    .into_iter()
                    .filter(|key| !METRICS.contains(&key.as_str()))
                    .collect();
                if invalids.is_empty() {
                    Ok(())
                } else {
                    Err(LogError::InvalidFilter(invalids))
                }
            }
        }
    }

    /// Checks to see if a log type T is enabled, without actually requiring an instance of T.
    /// This allows checking even when the event is a callback.
    pub fn enabled<T: LogEvent>(&self) -> bool {
        match self {
            LogFilter::AllowAll => true,
            LogFilter::Some(filter) if filter.contains(T::NAME) => true,
            LogFilter::Some(_) => false,
        }
    }
}
