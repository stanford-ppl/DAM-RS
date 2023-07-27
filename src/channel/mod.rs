mod channel_id;
pub mod utils;
pub use channel_id::*;

mod events;

mod flavors;

pub use flavors::*;

pub mod channel_spec;
mod receiver;
mod sender;

pub(crate) mod handle;

use std::sync::Arc;


use crate::context::Context;
use crate::types::Cleanable;
use crate::types::DAMType;
use dam_core::*;

use dam_core::time::Time;
use dam_macros::log_producer;

use self::events::ReceiverEvent;
use self::events::SendEvent;
use self::handle::ChannelData;
use self::handle::ChannelHandle;
use self::receiver::{ReceiverFlavor, ReceiverImpl};
use dam_core::metric::LogProducer;

use self::sender::{SendOptions, SenderFlavor, SenderImpl};

#[derive(Clone, Debug)]
pub struct ChannelElement<T> {
    pub time: Time,
    pub data: T,
}

impl<T: Clone> ChannelElement<T> {
    pub fn new(time: Time, data: T) -> ChannelElement<T> {
        ChannelElement { time, data }
    }

    pub fn update_time(&mut self, new_time: Time) {
        self.time = std::cmp::max(self.time, new_time);
    }
}

#[derive(Clone, Debug)]
pub enum Recv<T> {
    Something(ChannelElement<T>),
    Nothing(Time),
    Closed,
    Unknown,
}

#[log_producer]
pub struct Sender<T: Clone> {
    pub(crate) underlying: Arc<ChannelData<T>>,
}

impl<T: DAMType> Sender<T> {
    pub fn id(&self) -> ChannelID {
        self.underlying.id()
    }

    pub fn attach_sender(&self, sender: &dyn Context) {
        // Self::log(SendEvent::AttachSender(self.id, sender.id()));
        self.under().attach_sender(sender);
    }

    pub fn try_send(&mut self, data: ChannelElement<T>) -> Result<(), SendOptions> {
        Self::log(SendEvent::TrySend(self.id()));
        self.under().try_send(data)
    }

    pub fn enqueue(
        &mut self,
        manager: &mut TimeManager,
        data: ChannelElement<T>,
    ) -> Result<(), EnqueueError> {
        Self::log(SendEvent::EnqueueStart(self.id()));
        let res = self.under().enqueue(manager, data);
        Self::log(SendEvent::EnqueueFinish(self.id()));
        res
    }
}

impl<T: Clone> Sender<T> {
    fn under(&self) -> &mut SenderImpl<T> {
        self.underlying.sender()
    }
}

impl<T: Clone> Cleanable for Sender<T> {
    fn cleanup(&mut self) {
        self.under().cleanup();
        // Self::log(SendEvent::Cleanup(self.id));
    }
}
impl<T: Clone> Drop for Sender<T> {
    fn drop(&mut self) {
        self.cleanup();
    }
}

#[log_producer]
pub struct Receiver<T: Clone> {
    pub(crate) underlying: Arc<ChannelData<T>>,
}

impl<T: DAMType> Receiver<T> {
    pub fn id(&self) -> ChannelID {
        self.underlying.id()
    }

    pub fn attach_receiver(&self, receiver: &dyn Context) {
        Self::log(ReceiverEvent::AttachReceiver(self.id(), receiver.id()));
        self.under().attach_receiver(receiver)
    }

    pub fn peek(&mut self) -> Recv<T> {
        Self::log(ReceiverEvent::Peek(self.id()));
        self.under().peek()
    }
    pub fn peek_next(&mut self, manager: &mut TimeManager) -> Recv<T> {
        Self::log(ReceiverEvent::PeekNextStart(self.id()));
        let result = self.under().peek_next(manager);
        Self::log(ReceiverEvent::PeekNextFinish(self.id()));
        result
    }

    pub fn dequeue(&mut self, manager: &mut TimeManager) -> Recv<T> {
        Self::log(ReceiverEvent::DequeueStart(self.id()));
        let result = self.under().dequeue(manager);
        Self::log(ReceiverEvent::DequeueFinish(self.id()));
        result
    }
}

impl<T: DAMType> Cleanable for Receiver<T> {
    fn cleanup(&mut self) {
        self.under().cleanup();
        Self::log(ReceiverEvent::Cleanup(self.id()));
    }
}

impl<T: DAMType> Receiver<T> {
    fn under(&self) -> &mut ReceiverImpl<T> {
        self.underlying.receiver()
    }
}

#[derive(Debug)]
pub struct DequeueError {}

impl std::error::Error for DequeueError {}

impl std::fmt::Display for DequeueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Attempted to dequeue from simulation-closed channel!")
    }
}

#[derive(Debug)]
pub struct EnqueueError {}
impl std::error::Error for EnqueueError {}

impl std::fmt::Display for EnqueueError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Attempted to enqueue to a simulation-closed channel!")
    }
}
