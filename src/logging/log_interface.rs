use crossbeam::channel::Sender;

use super::{LogEntry, LogError, LogEvent, LogFilter};
use crate::datastructures::Identifier;
use derive_more::Constructor;

#[derive(Clone, Constructor)]
pub struct LogInterface {
    comm: Sender<LogEntry>,
    pub id: Identifier,
    base_time: std::time::Instant,
    pub log_filter: LogFilter,
}

impl LogInterface {
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
