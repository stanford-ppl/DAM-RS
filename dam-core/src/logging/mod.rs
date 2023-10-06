use std::{cell::RefCell, num::TryFromIntError, sync::Arc};

use mongodb::{
    bson::{self, Bson},
    Client, Collection,
};
use serde::{Deserialize, Serialize};

mod logger;
pub use logger::*;

// Mongodb logging structure
// A programgraph -> one Database
// A thread -> one collection

use thiserror::Error;

#[derive(Error, Debug)]
pub enum LogError {
    #[error("Serialization Error")]
    SerializationError(mongodb::bson::ser::Error),

    #[error("Error converting time into i64. Did we run out of time?")]
    TimeConversionError(TryFromIntError),

    #[error("Uninitialized logger!")]
    Uninitialized,
}

#[derive(Debug, Default, Clone)]
pub enum LogState {
    #[default]
    Disabled,
    Active(Logger),
}

#[derive(Debug, Clone)]
pub struct MongoLoggingOptions {
    pub client: Client,
    pub database: String,
    pub max_size: Option<u64>,
}

thread_local! {
    pub static LOGGER: RefCell<LogState> = RefCell::default();
}

#[inline]
pub fn log_event<T: Serialize, F>(callback: F) -> Result<(), LogError>
where
    F: FnOnce() -> T,
{
    LOGGER.with(|logger| match &*logger.borrow() {
        LogState::Disabled => {
            // If we're disabled, then don't do anything.
            Ok(())
        }
        LogState::Active(log) => log.log(callback()),
    })
}

pub fn initialize_log(logger: Logger) {
    LOGGER.with(|cur_logger| {
        let old = cur_logger.replace(LogState::Active(logger));
        assert!(matches!(old, LogState::Disabled));
    })
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
            context: 0,
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
