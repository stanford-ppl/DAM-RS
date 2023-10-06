use bson::Bson;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::num::TryFromIntError;
use thiserror::Error;

// Adds a logger that does nothing.
mod null_logger;
pub use null_logger::*;

#[cfg(feature = "log-mongo")]
mod mongo_logger;
#[cfg(feature = "log-mongo")]
pub use mongo_logger::*;

mod log_interface;
pub use log_interface::LogInterface;

mod log_functions;
pub use log_functions::*;

use self::registry::{get_metrics_vec, METRICS};

pub mod registry;

#[derive(Error, Debug)]
pub enum LogError {
    #[error("Error converting time into i64. Did we run out of time?")]
    TimeConversionError(TryFromIntError),

    #[error("Could not send event! Were LogProcessors registered?")]
    SendError,

    #[error(
        "Invalid Log Filter Defined: {0:?} were not registered filters! Options: {:?}",
        get_metrics_vec()
    )]
    InvalidFilter(Vec<String>),

    #[cfg(feature = "log-mongo")]
    #[error("Serialization Error")]
    SerializationError(mongodb::bson::ser::Error),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogEntry {
    // Time in microseconds since start of simulation
    pub timestamp: i64,

    // Identity of the current context
    pub context: usize,

    // String name of the logging event type
    pub event_type: String,

    // The actual data of the event
    pub event_data: Bson,
}

pub trait LogEvent: Serialize {
    const NAME: &'static str;
}

// Log Processors actually run and write the logs somewhere.
pub trait LogProcessor: Send {
    fn spawn(&mut self);
}

#[derive(Debug, Default, Clone)]
pub enum LogFilter {
    #[default]
    AllowAll,
    Some(FxHashSet<String>),
}

impl LogFilter {
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

    pub fn enabled<T: LogEvent>(&self) -> bool {
        match self {
            LogFilter::AllowAll => true,
            LogFilter::Some(filter) if filter.contains(T::NAME) => true,
            LogFilter::Some(_) => false,
        }
    }
}
