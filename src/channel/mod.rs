//! Channels in DAM are Single-Producer Single-Consumer (SPSC) constructs, and are the primary form of communication between [super::context::Context]s.
//! Blocking operations automatically handle time manipulation when used with blocking operations such as dequeue and enqueue.

mod channel_id;

/// Utility functions and constructs for interacting with channels.
pub mod utils;
pub use channel_id::*;

mod events;

mod flavors;

pub(crate) use flavors::*;

pub(crate) mod channel_spec;
mod receiver;
mod sender;

pub(crate) mod handle;

pub mod adapters;

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

/// An item with an associated timestamp -- used for sending/receiving objects on channels and modifying contexts' owned times.
#[derive(Clone, Debug)]
pub struct ChannelElement<T> {
    /// The element's timestamp
    pub time: Time,
    /// The contained data
    pub data: T,
}

impl<T> ChannelElement<T> {
    // TODO: Is this actually necessary?
    /// Constructs a new timestamp.
    pub fn new(time: Time, data: T) -> ChannelElement<T> {
        ChannelElement { time, data }
    }

    /// Updates the timestamp with a later timestamp. This is used for emulating stalls.
    pub fn update_time(&mut self, new_time: Time) {
        self.time = std::cmp::max(self.time, new_time);
    }

    /// Converts between ChannelElement types, where the underlying types are compatible.
    /// We can't blanket implement this via From/Into because there are existing impls
    pub fn convert<U>(self) -> ChannelElement<U>
    where
        T: Into<U>,
    {
        ChannelElement {
            time: self.time,
            data: self.data.into(),
        }
    }

    /// Attempts to convert between ChannelElement types.
    pub fn try_convert<U>(self) -> Result<ChannelElement<U>, <T as TryInto<U>>::Error>
    where
        T: TryInto<U>,
    {
        Ok(ChannelElement {
            time: self.time,
            data: self.data.try_into()?,
        })
    }
}

/// The result of a Peek operation
#[derive(Clone, Debug)]
pub enum PeekResult<T> {
    /// We found an element. Note: The timestamp MAY be in the future.
    Something(ChannelElement<T>),

    /// Nothing was available at a particular time -- also serves as proof that no element will arrive prior to or at the timestamp.
    Nothing(Time),

    /// Channel was closed. Roughly equivalent to Nothing(Infinity)
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

/// The send side of a channel, modelled after std::mpsc, crossbeam, and the like.
pub struct Sender<T: Clone> {
    pub(crate) underlying: Arc<ChannelData<T>>,
}

impl<T: DAMType> Sender<T> {
    /// Gets the ID of the channel.
    pub fn id(&self) -> ChannelID {
        self.underlying.id()
    }

    /// Registers a context for the sender.
    pub fn attach_sender(&self, sender: &dyn Context) {
        // log_event(&{SendEvent::AttachSender(self.id, sender.id())});
        if let SenderImpl::Uninitialized(uninit) = self.under() {
            uninit.attach_sender(sender);
        } else {
            panic!("Cannot attach a context to an initialized sender!");
        }
    }

    /// Writes to a channel. This will error if the receive side has already been closed.
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

    /// Advances time forward until the channel is not full.
    pub fn wait_until_available(&self, manager: &mut TimeManager) -> Result<(), EnqueueError> {
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

/// The receive side of a channel, modelled after std::mpsc, crossbeam, and the like.
pub struct Receiver<T: Clone> {
    pub(crate) underlying: Arc<ChannelData<T>>,
}

impl<T: DAMType> Receiver<T> {
    /// Gets the ID of the channel.
    pub fn id(&self) -> ChannelID {
        self.underlying.id()
    }

    /// Registers a context for the receiver.
    pub fn attach_receiver(&self, receiver: &dyn Context) {
        log_event(&ReceiverEvent::AttachReceiver(self.id(), receiver.id())).unwrap();
        if let ReceiverImpl::Uninitialized(recv) = self.under() {
            recv.attach_receiver(receiver);
        } else {
            panic!("Should not be able to attach a context to an initialized receiver")
        }
    }

    /// Peeks the channel. Note: It is possible to see a value in the future when peeking, as noted by [PeekResult].
    pub fn peek(&self) -> PeekResult<T> {
        log_event(&ReceiverEvent::Peek(self.id())).unwrap();
        self.under().peek()
    }

    /// Advances forward in time until there is an element in the channel, and returns that value.
    /// If the channel is closed before another element is sent, then it returns a DequeueError instead.
    pub fn peek_next(&self, manager: &TimeManager) -> Result<ChannelElement<T>, DequeueError> {
        log_event(&ReceiverEvent::PeekNextStart(self.id())).unwrap();
        let result = self.under().peek_next(manager);
        log_event(&ReceiverEvent::PeekNextFinish(self.id())).unwrap();
        result
    }

    /// Advances forward in time until there is an element in the channel, and pops that value.
    /// If the channel is closed before another element is sent, then it returns a DequeueError instead.
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

/// Errors that can occur when dequeueing from a channel.
#[derive(Error, Debug)]
pub enum DequeueError {
    /// Marks that the channel was closed without any further values.
    #[error("Dequeued from a simulation-closed channel!")]
    Closed,
}

/// Errors that can occur when enqueueing into a channel.
#[derive(Error, Debug)]
pub enum EnqueueError {
    /// Marks that the channel was closed without any further values.
    #[error("Enqueued to a simulation-closed channel!")]
    Closed,
}
