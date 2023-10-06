use crossbeam::channel::TryRecvError;

use super::LogEntry;
use derive_more::Constructor;

pub use mongodb;

#[derive(Clone, Constructor)]
pub struct MongoLogger {
    client: mongodb::sync::Client,
    database_name: String,
    db_options: mongodb::options::DatabaseOptions,
    collection_name: String,
    queue: crossbeam::channel::Receiver<LogEntry>,
}

impl super::LogProcessor for MongoLogger {
    fn spawn(&mut self) {
        let database = self
            .client
            .database_with_options(self.database_name.as_str(), self.db_options.clone());
        let collection = database.collection::<LogEntry>(self.collection_name.as_str());
        let mut should_continue = true;
        while should_continue {
            let mut batch = vec![];

            loop {
                match self.queue.try_recv() {
                    Ok(data) => batch.push(data),
                    Err(TryRecvError::Empty) => {
                        break;
                    }
                    Err(TryRecvError::Disconnected) => {
                        should_continue = false;
                        println!("Logger Disconnected");
                        break;
                    }
                }
            }
            if !batch.is_empty() {
                collection.insert_many(batch, None).unwrap();
            }
        }
        self.client.clone().shutdown();
        println!("Finished with logging!")
    }
}
