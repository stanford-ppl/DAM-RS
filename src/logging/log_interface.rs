use crossbeam::channel::Sender;

use super::{LogEntry, LogError, LogEvent, LogFilter};
use crate::datastructures::Identifier;
use derive_more::Constructor;

/// A logging interface, which simply pushes data into a communication channel.
/// Actual logging is done by the log processor.
#[derive(Clone, Constructor)]
pub struct LogInterface {
    /// The Identifier for the currently executing context
    pub id: Identifier,
    comm: Sender<LogEntry>,
    base_time: std::time::Instant,
    pub(crate) log_filter: LogFilter,
}

impl LogInterface {
    /// Logs an event into the communication channel.
    /// May return an error if either the channel was prematurely closed, or if some aspect of serialization failed.
    pub fn log<T: LogEvent>(&self, event: &T) -> Result<(), LogError> {
        self.comm
            .send(LogEntry {
                timestamp: self
                    .base_time
                    .elapsed()
                    .as_micros()
                    .try_into()
                    .map_err(|err| LogError::TimeConversionError(err))?,
                context: self.id.id,
                event_type: T::NAME.to_string(),
                event_data: bson::to_bson(event)
                    .map_err(|err| LogError::SerializationError(err))?,
            })
            .map_err(|_| LogError::SendError)?;

        Ok(())
    }
}
