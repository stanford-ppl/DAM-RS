use std::{num::TryFromIntError, sync::Arc};

use mongodb::{
    bson::{self, Bson},
    Collection,
};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

use crate::datastructures::Identifier;

use super::LogError;

// // We'll make one logger and pass it around.
// #[derive(Debug, Clone)]
// pub struct Logger {
//     collection: Collection<LogEntry>,
//     base: std::time::Instant,
//     runtime: Arc<tokio::runtime::Runtime>,
//     pub context: Identifier,
// }

// impl Logger {
//     pub fn new(
//         runtime: Arc<Runtime>,
//         base: std::time::Instant,
//         collection: Collection<LogEntry>,
//         context: Identifier,
//     ) -> Self {
//         Self {
//             collection,
//             base,
//             runtime,
//             context,
//         }
//     }

//     pub fn log<T: Serialize>(&self, obj: T) -> Result<(), LogError> {
//         let serialized = bson::to_bson(&obj).map_err(|err| LogError::SerializationError(err))?;
//         let entry = LogEntry {
//             timestamp: self
//                 .base
//                 .elapsed()
//                 .as_micros()
//                 .try_into()
//                 .map_err(|err| LogError::TimeConversionError(err))?,
//             event_type: std::any::type_name::<T>().to_string(),
//             event_data: serialized,
//             context: self.context.id,
//         };

//         let col = self.collection.clone();

//         self.runtime
//             .spawn(async move { col.insert_one(entry, None).await.unwrap() });

//         Ok(())
//     }

//     pub fn shutdown(self) {
//         self.runtime
//             .block_on(self.collection.client().clone().shutdown())
//     }
// }
