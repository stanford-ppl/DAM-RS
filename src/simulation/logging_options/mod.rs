pub mod mongo;

#[derive(Default)]
pub enum LoggingOptions {
    #[default]
    None,

    Mongo(mongo::MongoOptions),
}
