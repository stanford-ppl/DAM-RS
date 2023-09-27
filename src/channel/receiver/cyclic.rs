use dam_core::TimeManager;

use crate::channel::{DequeueResult, PeekResult};

use super::ReceiverCommon;

pub(super) trait CyclicReceiver<T: Clone>: ReceiverCommon<T> {
    fn peek_next(&mut self, manager: &mut TimeManager) -> DequeueResult<T> {
        loop {
            match self.peek() {
                PeekResult::Nothing(time) => manager.advance(time + 1), // Nothing here, so tick forward until there might be
                PeekResult::Closed => return DequeueResult::Closed, // Channel is closed, so let the dequeuer know
                PeekResult::Something(stuff) => {
                    manager.advance(stuff.time);
                    return DequeueResult::Something(stuff);
                }
            }
        }
    }

    fn dequeue(&mut self, manager: &mut TimeManager) -> DequeueResult<T> {
        let result = self.peek_next(manager);
        match result {
            DequeueResult::Something(data) => {
                self.register_recv(data.time);
                self.data().head = None;
                return DequeueResult::Something(data);
            }
            DequeueResult::Closed => return result,
        }
    }
}
