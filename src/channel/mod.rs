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
use thiserror::Error;

use crate::context::Context;

use crate::datastructures::Time;
use crate::logging::log_event;
use crate::types::DAMType;
use crate::view::TimeManager;

use self::events::ReceiverEvent;
use self::events::SendEvent;
use self::handle::ChannelData;
use self::handle::ChannelHandle;

use self::receiver::terminated::TerminatedReceiver;
use self::receiver::{ReceiverFlavor, ReceiverImpl};
use self::sender::terminated::TerminatedSender;

use self::sender::{SenderFlavor, SenderImpl};

#[derive(Clone, Debug)]
pub struct ChannelElement<T> {
    pub time: Time,
    pub data: T,
}

impl<T: Clone> ChannelElement<T> {
    pub fn new(time: Time, data: T) -> ChannelElement<T> {
        ChannelElement { time, data }
    }
}

impl<T> ChannelElement<T> {
    pub fn update_time(&mut self, new_time: Time) {
        self.time = std::cmp::max(self.time, new_time);
    }
}

#[derive(Clone, Debug)]
pub enum PeekResult<T> {
    Something(ChannelElement<T>),
    Nothing(Time),
    Closed,
}

impl<T> TryInto<Result<ChannelElement<T>, DequeueError>> for PeekResult<T> {
    type Error = ();

    fn try_into(self) -> Result<Result<ChannelElement<T>, DequeueError>, Self::Error> {
        match self {
            PeekResult::Something(data) => Ok(Ok(data)),
            PeekResult::Nothing(_) => Err(()),
            PeekResult::Closed => Ok(Err(DequeueError::Closed)),
        }
    }
}

pub struct Sender<T: Clone> {
    pub(crate) underlying: Arc<ChannelData<T>>,
}

impl<T: DAMType> Sender<T> {
    pub fn id(&self) -> ChannelID {
        self.underlying.id()
    }

    pub fn attach_sender(&self, sender: &dyn Context) {
        // log_event(&{SendEvent::AttachSender(self.id, sender.id())});
        if let SenderImpl::Uninitialized(uninit) = self.under() {
            uninit.attach_sender(sender);
        } else {
            panic!("Cannot attach a context to an initialized sender!");
        }
    }
    pub fn enqueue(
        &self,
        manager: &TimeManager,
        data: ChannelElement<T>,
    ) -> Result<(), EnqueueError> {
        log_event(&SendEvent::EnqueueStart(self.id())).unwrap();
        let res = self.under().enqueue(manager, data);
        log_event(&SendEvent::EnqueueFinish(self.id())).unwrap();
        res
    }

    pub fn wait_until_available(&mut self, manager: &mut TimeManager) -> Result<(), EnqueueError> {
        self.under().wait_until_available(manager)
    }
}

impl<T: Clone> Drop for Sender<T> {
    fn drop(&mut self) {
        *self.under() = TerminatedSender::default().into();
    }
}

impl<T: Clone> Sender<T> {
    fn under(&self) -> &mut SenderImpl<T> {
        self.underlying.sender()
    }
}

pub struct Receiver<T: Clone> {
    pub(crate) underlying: Arc<ChannelData<T>>,
}

impl<T: DAMType> Receiver<T> {
    pub fn id(&self) -> ChannelID {
        self.underlying.id()
    }

    pub fn attach_receiver(&self, receiver: &dyn Context) {
        log_event(&ReceiverEvent::AttachReceiver(self.id(), receiver.id())).unwrap();
        if let ReceiverImpl::Uninitialized(recv) = self.under() {
            recv.attach_receiver(receiver);
        } else {
            panic!("Should not be able to attach a context to an initialized receiver")
        }
    }

    pub fn peek(&self) -> PeekResult<T> {
        log_event(&ReceiverEvent::Peek(self.id())).unwrap();
        self.under().peek()
    }
    pub fn peek_next(&self, manager: &TimeManager) -> Result<ChannelElement<T>, DequeueError> {
        log_event(&ReceiverEvent::PeekNextStart(self.id())).unwrap();
        let result = self.under().peek_next(manager);
        log_event(&ReceiverEvent::PeekNextFinish(self.id())).unwrap();
        result
    }

    pub fn dequeue(&self, manager: &TimeManager) -> Result<ChannelElement<T>, DequeueError> {
        log_event(&ReceiverEvent::DequeueStart(self.id())).unwrap();
        let result = self.under().dequeue(manager);
        log_event(&ReceiverEvent::DequeueFinish(self.id())).unwrap();
        result
    }
}

impl<T: Clone> Receiver<T> {
    fn under(&self) -> &mut ReceiverImpl<T> {
        self.underlying.receiver()
    }
}

impl<T: Clone> Drop for Receiver<T> {
    fn drop(&mut self) {
        *self.under() = TerminatedReceiver::default().into();
    }
}

#[derive(Error, Debug)]
pub enum DequeueError {
    #[error("Dequeued from a simulation-closed channel!")]
    Closed,
}

#[derive(Error, Debug)]
pub enum EnqueueError {
    #[error("Enqueued to a simulation-closed channel!")]
    Closed,
}
