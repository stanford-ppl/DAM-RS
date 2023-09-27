use dam_core::TimeManager;

use crate::channel::{DequeueResult, PeekResult};

use super::ReceiverFlavor;

pub struct TerminatedReceiver {}

impl TerminatedReceiver {}

impl<T> ReceiverFlavor<T> for TerminatedReceiver {
    fn peek(&mut self) -> PeekResult<T> {
        panic!();
    }

    fn peek_next(&mut self, _manager: &mut TimeManager) -> DequeueResult<T> {
        panic!();
    }

    fn dequeue(&mut self, _manager: &mut TimeManager) -> DequeueResult<T> {
        panic!();
    }
}
