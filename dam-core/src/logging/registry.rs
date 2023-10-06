// Re-export distributed_slice for external use.
pub use linkme::distributed_slice;

#[distributed_slice]
pub static METRICS: [&'static str] = [..];
