use crate::{
    channel::{ChannelElement, DequeueError, PeekResult},
    view::TimeManager,
};

use super::ReceiverCommon;

pub(super) trait AcyclicReceiver<T: Clone>: ReceiverCommon<T> {
    fn peek_next(&mut self, manager: &TimeManager) -> Result<ChannelElement<T>, DequeueError> {
        match &self.data().head {
            Some(PeekResult::Closed) => return Err(DequeueError::Closed),
            None | Some(PeekResult::Nothing(_)) => {}
            Some(PeekResult::Something(data)) => return Ok(data.clone()),
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

    fn dequeue(&mut self, manager: &TimeManager) -> Result<ChannelElement<T>, DequeueError> {
        match self.data().head {
            Some(PeekResult::Closed) => return Err(DequeueError::Closed),
            Some(PeekResult::Something(_)) => {
                let PeekResult::Something(element) = self.data().head.take().unwrap() else {
                    unreachable!();
                };
                self.data().head = None;
                manager.advance(element.time);
                self.register_recv(element.time.max(manager.tick()));
                return Ok(element);
            }
            None | Some(PeekResult::Nothing(_)) => {}
        }

        // At this point, we can just block!
        match self.data().underlying.recv() {
            Ok(ce) => {
                self.register_recv(ce.time.max(manager.tick()));
                manager.advance(ce.time);
                Ok(ce)
            }
            Err(_) => {
                self.data().head = Some(PeekResult::Closed);
                Err(DequeueError::Closed)
            }
        }
    }
}
