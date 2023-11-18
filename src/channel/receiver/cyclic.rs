use crate::{
    channel::{ChannelElement, DequeueError, PeekResult},
    view::TimeManager,
};

use super::ReceiverCommon;

pub(super) trait CyclicReceiver<T: Clone>: ReceiverCommon<T> {
    fn peek_next(&mut self, manager: &TimeManager) -> Result<ChannelElement<T>, DequeueError> {
        loop {
            match self.peek() {
                PeekResult::Nothing(time) => manager.advance(time + 1), // Nothing here, so tick forward until there might be
                PeekResult::Closed => return Err(DequeueError::Closed), // Channel is closed, so let the dequeuer know
                PeekResult::Something(stuff) => {
                    manager.advance(stuff.time);
                    return Ok(stuff);
                }
            }
        }
    }

    fn dequeue(&mut self, manager: &TimeManager) -> Result<ChannelElement<T>, DequeueError> {
        let result = self.peek_next(manager);
        match result {
            Ok(data) => {
                self.register_recv(data.time.max(manager.tick()));
                self.data().head = None;
                Ok(data)
            }
            Err(DequeueError::Closed) => result,
        }
    }
}
