//! Defines a set of adapters for converting between channel types at a type-level.
//! In particular, these are useful when some memory may contain elements of different types
//! And so channels of different types may be connected to the memory.

use crate::{context::Context, context_tools::DAMType, structures::TimeManager};

use super::{ChannelElement, DequeueError, EnqueueError, PeekResult, Receiver, Sender};

/// An adapter for Receivers, delegating and converting all underlying operations
pub trait RecvAdapter<U> {
    /// See: [Receiver::attach_receiver]
    fn attach_receiver(&self, ctx: &dyn Context);

    /// See: [Receiver::peek]
    fn peek(&self) -> PeekResult<U>;
    /// See: [Receiver::peek_next]
    fn peek_next(&self, manager: &TimeManager) -> Result<ChannelElement<U>, DequeueError>;
    /// See: [Receiver::dequeue]
    fn dequeue(&self, manager: &TimeManager) -> Result<ChannelElement<U>, DequeueError>;
}

impl<T: DAMType, U> RecvAdapter<U> for Receiver<T>
where
    T: TryInto<U>,
{
    fn attach_receiver(&self, ctx: &dyn Context) {
        Receiver::attach_receiver(self, ctx)
    }

    fn peek(&self) -> PeekResult<U> {
        match Receiver::peek(self) {
            PeekResult::Something(ce) => {
                PeekResult::Something(ce.try_convert().unwrap_or_else(|_| {
                    panic!("Failed to convert the peek value into the desired type")
                }))
            }
            PeekResult::Nothing(time) => PeekResult::Nothing(time),
            PeekResult::Closed => PeekResult::Closed,
        }
    }

    fn peek_next(&self, manager: &TimeManager) -> Result<ChannelElement<U>, DequeueError> {
        Receiver::peek_next(self, manager).map(|val| {
            val.try_convert().unwrap_or_else(|_| {
                panic!("Failed to convert the peek_next value into the desired type")
            })
        })
    }

    fn dequeue(&self, manager: &TimeManager) -> Result<ChannelElement<U>, DequeueError> {
        Receiver::dequeue(self, manager).map(|val| {
            val.try_convert().unwrap_or_else(|_| {
                panic!("Failed to convert the dequeued value into the desired type")
            })
        })
    }
}

/// An adapter for Senders, delegating and converting all underlying operations.
pub trait SendAdapter<U> {
    /// See: [Sender::attach_sender]
    fn attach_sender(&self, ctx: &dyn Context);

    /// See: [Sender::enqueue]
    fn enqueue(&self, manager: &TimeManager, data: ChannelElement<U>) -> Result<(), EnqueueError>;

    /// See: [Sender::wait_until_available]
    fn wait_until_available(&self, manager: &TimeManager) -> Result<(), EnqueueError>;
}

impl<T: DAMType, U> SendAdapter<U> for Sender<T>
where
    T: From<U>,
{
    fn enqueue(&self, manager: &TimeManager, data: ChannelElement<U>) -> Result<(), EnqueueError> {
        Sender::enqueue(
            self,
            manager,
            data.try_convert().unwrap_or_else(|_| {
                panic!("Failed to convert the enqueued value into the desired type")
            }),
        )
    }

    fn wait_until_available(&self, manager: &TimeManager) -> Result<(), EnqueueError> {
        Sender::wait_until_available(self, manager)
    }

    fn attach_sender(&self, ctx: &dyn Context) {
        Sender::attach_sender(self, ctx)
    }
}
