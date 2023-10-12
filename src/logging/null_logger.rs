use super::LogProcessor;

/// A no-op logger.
#[derive(Clone)]
pub struct NullLogger {}

impl LogProcessor for NullLogger {
    fn spawn(&mut self) {} // Does nothing.
}
