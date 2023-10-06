use dam_core::logging::mongodb;
use derive_builder::Builder;

#[derive(Clone, Debug, Builder)]
#[builder(pattern = "owned")]
pub struct MongoOptions {
    pub uri: String,

    #[builder(default)]
    pub db_options: mongodb::options::DatabaseOptions,

    pub db: String,

    #[builder(default = "\"log\".to_string()")]
    pub collection: String,
}
