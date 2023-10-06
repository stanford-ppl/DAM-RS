use std::{cell::RefCell, num::TryFromIntError};

use crossbeam::channel::Sender;
use mongodb::bson::{self, Bson};
use serde::{Deserialize, Serialize};

mod mongo_logger;
mod nullprocessor;

// Mongodb logging structure
// A programgraph -> one Database
// A thread -> one collection

use thiserror::Error;

use crate::datastructures::Identifier;

pub use mongo_logger::*;
pub use nullprocessor::*;

#[derive(Error, Debug)]
pub enum LogError {
    #[error("Serialization Error")]
    SerializationError(mongodb::bson::ser::Error),

    #[error("Error converting time into i64. Did we run out of time?")]
    TimeConversionError(TryFromIntError),

    #[error("Could not send event! Were LogProcessors registered?")]
    SendError,
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

pub struct LogInterface {
    comm: Sender<LogEntry>,
    id: Identifier,
    base_time: std::time::Instant,
}

impl LogInterface {
    pub fn new(id: Identifier, base_time: std::time::Instant, comm: Sender<LogEntry>) -> Self {
        Self {
            id,
            comm,
            base_time,
        }
    }

    pub fn log<T: Serialize>(&self, event: &T) -> Result<(), LogError> {
        self.comm
            .send(LogEntry {
                timestamp: self
                    .base_time
                    .elapsed()
                    .as_micros()
                    .try_into()
                    .map_err(|err| LogError::TimeConversionError(err))?,
                context: self.id.id,
                event_type: std::any::type_name::<T>().to_string(),
                event_data: bson::to_bson(event)
                    .map_err(|err| LogError::SerializationError(err))?,
            })
            .map_err(|_| LogError::SendError)?;

        Ok(())
    }
}

// Log Processors actually run and write the logs somewhere.
pub trait LogProcessor: Send {
    fn spawn(&mut self);
}

thread_local! {
    pub static LOGGER: RefCell<Option<LogInterface>> = RefCell::default();
}

#[inline]
pub fn log_event<T: Serialize, F>(callback: F) -> Result<(), LogError>
where
    F: FnOnce() -> T,
{
    LOGGER.with(|logger| match &*logger.borrow() {
        Some(interface) => interface.log(&callback()),
        None => Ok(()),
    })
}

pub fn initialize_log(logger: LogInterface) {
    LOGGER.with(|cur_logger| {
        let old = cur_logger.replace(Some(logger));
        assert!(matches!(old, None));
    })
}

pub fn destroy_log() {
    LOGGER.with(|cur_logger| {
        cur_logger.take();
    })
}
