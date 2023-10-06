use std::{num::TryFromIntError, sync::Arc};

use mongodb::{
    bson::{self, Bson},
    Collection,
};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use crate::datastructures::Identifier;

use super::{LogError, MongoLoggingOptions};

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

// We'll make one logger and pass it around.
#[derive(Debug, Clone)]
pub struct Logger {
    collection: Collection<LogEntry>,
    base: std::time::Instant,
    runtime: Arc<tokio::runtime::Runtime>,
    context: Identifier,
}

impl Logger {
    pub fn new(
        runtime: Arc<Runtime>,
        base: std::time::Instant,
        options: MongoLoggingOptions,
        context: Identifier,
    ) -> Self {
        let db = options.client.database(&options.database);
        Self {
            collection: db.collection("event_log"),
            base,
            runtime,
            context,
        }
    }

    pub fn log<T: Serialize>(&self, obj: T) -> Result<(), LogError> {
        let serialized = bson::to_bson(&obj).map_err(|err| LogError::SerializationError(err))?;
        let entry = LogEntry {
            timestamp: self
                .base
                .elapsed()
                .as_micros()
                .try_into()
                .map_err(|err| LogError::TimeConversionError(err))?,
            event_type: std::any::type_name::<T>().to_string(),
            event_data: serialized,
            context: self.context.id,
        };

        let col = self.collection.clone();

        self.runtime
            .spawn(async move { col.insert_one(entry, None).await.unwrap() });

        Ok(())
    }
}
