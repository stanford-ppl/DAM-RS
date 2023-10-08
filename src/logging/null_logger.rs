use super::LogProcessor;

#[derive(Clone)]
pub struct NullLogger {}

impl LogProcessor for NullLogger {
    fn spawn(&mut self) {} // Does nothing.
}
