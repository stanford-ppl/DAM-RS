use linkme::distributed_slice;

pub trait LogProducer {
    // This should be a simple name consisting only of [a-zA-Z] and "-" characters.
    const LOG_NAME: &'static str;

    fn log<T: serde::Serialize + std::fmt::Debug>(_value: T) {}
}

pub fn validate_name<'a>(name: &'a str) -> bool {
    METRICS.contains(&name)
}

#[distributed_slice]
pub static METRICS: [&'static str] = [..];

// Gathers all information about the nodes
#[distributed_slice(METRICS)]
pub static NODE: &'static str = "NODE";
