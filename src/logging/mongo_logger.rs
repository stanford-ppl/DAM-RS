//! This module provides MongoDB support for logging, and is the only currently supported logger.
//!
//! The MongoLogger takes in a [crossbeam::channel::Receiver] containing [LogEntry] and writes them to the database in as large a chunk as it can.

use futures::task::LocalSpawnExt;
use mongodb::options::{InsertManyOptions, WriteConcern};

use super::LogEntry;
use derive_more::Constructor;

pub use mongodb;

/// A logger using MongoDB as the backing datastore.
#[derive(Clone, Constructor)]
pub struct MongoLogger {
    client: mongodb::Client,
    database_name: String,
    db_options: mongodb::options::DatabaseOptions,
    collection_name: String,
    collection_options: mongodb::options::CreateCollectionOptions,
    queue: crossbeam::channel::Receiver<LogEntry>,
}

impl super::LogProcessor for MongoLogger {
    fn spawn(&mut self) {
        let database = self
            .client
            .database_with_options(self.database_name.as_str(), self.db_options.clone());

        futures::executor::block_on(database.create_collection(
            self.collection_name.as_str(),
            self.collection_options.clone(),
        ))
        .expect("Error setting collection options");

        let collection = database.collection::<LogEntry>(self.collection_name.as_str());

        let mut executor = futures::executor::LocalPool::new();
        let spawner = executor.spawner();
        let mut should_continue = true;
        while should_continue {
            let mut batch = vec![];
            loop {
                match self.queue.try_recv() {
                    Ok(data) => batch.push(data),
                    Err(crossbeam::channel::TryRecvError::Empty) => {
                        break;
                    }
                    Err(crossbeam::channel::TryRecvError::Disconnected) => {
                        should_continue = false;
                        break;
                    }
                }
            }
            if !batch.is_empty() {
                let col_clone = collection.clone();
                let boxed = Box::new(col_clone);
                let fut = async move {
                    boxed
                        .insert_many(
                            batch.iter(),
                            Some(
                                InsertManyOptions::builder()
                                    .write_concern(WriteConcern::builder().journal(false).build())
                                    .ordered(false)
                                    .build(),
                            ),
                        )
                        .await
                        .unwrap();
                    ()
                };

                spawner.spawn_local(fut).unwrap();
            }
            executor.run_until_stalled();
        }
        executor.run();
        futures::executor::block_on(self.client.clone().shutdown());
    }
}
