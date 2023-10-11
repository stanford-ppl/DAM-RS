use derive_builder::Builder;
use mongodb;

/// Options for a MongoDB-driven logging system
#[derive(Clone, Debug, Builder)]
#[builder(pattern = "owned")]
pub struct MongoOptions {
    /// The URI of the database, i.e. mongodb://localhost:27017
    pub uri: String,

    /// The options for the database
    #[builder(default)]
    pub db_options: mongodb::options::DatabaseOptions,

    /// Options to use for creating the collection, most notably the capped collection settings
    /// to limit how many log entries to save.
    #[builder(default)]
    pub col_options: mongodb::options::CreateCollectionOptions,

    /// Name of the database to use
    pub db: String,

    /// Name of the collection to log to -- by default the name is just "log"
    #[builder(default = "\"log\".to_string()")]
    pub collection: String,
}
