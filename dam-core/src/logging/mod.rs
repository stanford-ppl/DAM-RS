use std::{num::TryFromIntError, sync::Arc};

use mongodb::{
    bson::{self, Bson},
    Collection,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// Mongodb logging structure
// A programgraph -> one Database
// A thread -> one collection

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogEntry {
    // Time in microseconds since start of simulation
    pub timestamp: i64,

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
}

#[derive(Error, Debug)]
pub enum LogError {
    #[error("Serialization Error")]
    SerializationError(mongodb::bson::ser::Error),

    #[error("Error converting time into i64. Did we run out of time?")]
    TimeConversionError(TryFromIntError),
}

impl Logger {
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
        };

        let col = self.collection.clone();

        self.runtime
            .spawn(async move { col.insert_one(entry, None).await.unwrap() });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use mongodb::{
        bson::{self, doc},
        options::CreateCollectionOptions,
        Client,
    };
    use serde::{Deserialize, Serialize};

    use super::LogEntry;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct Book {
        title: String,
        author: String,
    }

    async fn run_mongodb() -> Result<(), mongodb::error::Error> {
        let client = Client::with_uri_str("mongodb://localhost:27017").await?;
        let database = client.database("mydb");
        let options = CreateCollectionOptions::builder()
            .size(1 << 12)
            .capped(true);
        database
            .create_collection("books", Some(options.build()))
            .await?;
        let collection = database.collection::<LogEntry>("books");

        let docs = vec![
            Book {
                title: "1984".to_string(),
                author: "George Orwell".to_string(),
            },
            Book {
                title: "Animal Farm".to_string(),
                author: "George Orwell".to_string(),
            },
            Book {
                title: "The Great Gatsby".to_string(),
                author: "F. Scott Fitzgerald".to_string(),
            },
        ];

        let mapped = docs.into_iter().map(|book| LogEntry {
            timestamp: 0,
            event_type: "book".to_string(),
            event_data: bson::to_bson(&book).unwrap(),
        });

        // Insert some books into the "mydb.books" collection.
        collection
            .insert_many(mapped.cycle().take(512), None)
            .await?;

        Ok(())
    }

    #[test]
    fn drive_mongodb() {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(run_mongodb())
            .unwrap();
    }
}
