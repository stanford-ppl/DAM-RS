use dam_core::prelude::*;

use crate::channel::{DequeueResult, PeekResult};

use super::ReceiverCommon;

pub(super) trait AcyclicReceiver<T: Clone>: ReceiverCommon<T> {
    fn peek_next(&mut self, manager: &TimeManager) -> DequeueResult<T> {
        match &self.data().head {
            Some(PeekResult::Closed) => return DequeueResult::Closed,
            None | Some(PeekResult::Nothing(_)) => {}
            Some(PeekResult::Something(data)) => return DequeueResult::Something(data.clone()),
        }

        self.data().head = match self.data().underlying.recv() {
            Ok(stuff) => {
                manager.advance(stuff.time);
                Some(PeekResult::Something(stuff))
            }
            Err(_) => Some(PeekResult::Closed),
        };
        self.data().head.clone().unwrap().try_into().unwrap()
    }

    fn dequeue(&mut self, manager: &TimeManager) -> DequeueResult<T> {
        match &self.data().head {
            Some(PeekResult::Closed) => return DequeueResult::Closed,
            Some(PeekResult::Something(element)) => {
                let cloned = element.clone();
                self.data().head = None;
                self.register_recv(cloned.time);
                manager.advance(cloned.time);
                return DequeueResult::Something(cloned);
            }
            None | Some(PeekResult::Nothing(_)) => {}
        }

        // At this point, we can just block!
        match self.data().underlying.recv() {
            Ok(ce) => {
                self.register_recv(ce.time);
                manager.advance(ce.time);
                DequeueResult::Something(ce)
            }
            Err(_) => {
                self.data().head = Some(PeekResult::Closed);
                DequeueResult::Closed
            }
        }
    }
}
