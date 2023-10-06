use super::LogProcessor;

#[derive(Clone)]
pub struct NullProcessor {}

impl LogProcessor for NullProcessor {
    fn spawn(&mut self) {} // Does nothing.
}
