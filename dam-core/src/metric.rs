use linkme::distributed_slice;

use crate::log_graph::get_log;

pub trait LogProducer {
    // This should be a simple name consisting only of [a-zA-Z] and "-" characters.
    const LOG_NAME: &'static str;

    fn log<T: serde::Serialize + std::fmt::Debug>(value: T) {
        get_log(Self::LOG_NAME).log(value);
    }
}

pub fn validate_name<'a>(name: &'a str) -> bool {
    METRICS.contains(&name)
}

#[distributed_slice]
pub static METRICS: [&'static str] = [..];

// Gathers all information about the nodes
#[distributed_slice(METRICS)]
pub static NODE: &'static str = "NODE";
