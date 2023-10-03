use dam_core::prelude::*;

use crate::channel::{DequeueResult, PeekResult};

use super::ReceiverFlavor;

#[derive(Default)]
pub struct TerminatedReceiver {}

impl TerminatedReceiver {}

impl<T> ReceiverFlavor<T> for TerminatedReceiver {
    fn peek(&mut self) -> PeekResult<T> {
        panic!("Calling peek on a terminated receiver");
    }

    fn peek_next(&mut self, _manager: &TimeManager) -> DequeueResult<T> {
        panic!("Calling peek_next on a terminated receiver");
    }

    fn dequeue(&mut self, _manager: &TimeManager) -> DequeueResult<T> {
        panic!("Calling dequeue on a terminated receiver");
    }
}
