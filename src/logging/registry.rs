// Re-export distributed_slice for external use.
pub use linkme::distributed_slice;

/// A registry of all enabled event types
#[distributed_slice]
pub static METRICS: [&'static str] = [..];

/// This function is used to get the list of metrics for printing.
/// Otherwise, you should just use the METRICS distributed slice.
pub fn get_metrics_vec() -> Vec<&'static str> {
    METRICS.iter().map(|x| *x).collect()
}
