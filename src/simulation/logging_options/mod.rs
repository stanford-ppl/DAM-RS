#[cfg(feature = "log-mongo")]
pub mod mongo;

#[derive(Default)]
pub enum LoggingOptions {
    #[default]
    None,

    #[cfg(feature = "log-mongo")]
    Mongo(mongo::MongoOptions),
}
