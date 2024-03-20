// #[cfg_attr(docsrs, doc(cfg(feature = "log-mongo")))]
#[cfg(feature = "log-mongo")]
mod mongo;
// #[cfg_attr(docsrs, doc(cfg(feature = "log-mongo")))]
#[cfg(feature = "log-mongo")]
pub use mongo::*;

/// This enum serves as a registry of all loggers that are currently enabled, and are gated by feature flags.
#[derive(Default, Clone)]
pub enum LoggingOptions {
    /// Disabled logs
    #[default]
    None,

    /// Log to MongoDB
    // #[cfg_attr(docsrs, doc(cfg(feature = "log-mongo")))]
    #[cfg(feature = "log-mongo")]
    Mongo(MongoOptions),
}
