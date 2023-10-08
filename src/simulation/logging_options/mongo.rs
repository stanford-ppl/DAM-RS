use derive_builder::Builder;
use mongodb;

#[derive(Clone, Debug, Builder)]
#[builder(pattern = "owned")]
pub struct MongoOptions {
    pub uri: String,

    #[builder(default)]
    pub db_options: mongodb::options::DatabaseOptions,

    #[builder(default)]
    pub col_options: mongodb::options::CreateCollectionOptions,

    pub db: String,

    #[builder(default = "\"log\".to_string()")]
    pub collection: String,
}
