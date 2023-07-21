mod channel_id;
pub mod utils;
pub use channel_id::*;

mod events;

mod flavors;
pub use flavors::*;

mod receiver;
mod sender;
mod view_struct;

use std::sync::Arc;

use crate::context::Context;
use crate::types::Cleanable;
use crate::types::DAMType;
use crossbeam::channel;
use dam_core::*;

use dam_core::metric::LogProducer;
use dam_core::time::Time;
use dam_macros::log_producer;

use self::events::ReceiverEvent;
use self::events::SendEvent;
use self::receiver::AcyclicReceiver;
use self::receiver::CyclicReceiver;
use self::receiver::InfiniteReceiver;
use self::receiver::{ReceiverFlavor, ReceiverImpl};
use self::sender::AcyclicSender;
use self::sender::CyclicSender;
use self::sender::InfiniteSender;
use self::sender::VoidSender;
use self::sender::{SendOptions, SenderFlavor, SenderImpl};
use self::view_struct::ViewStruct;

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
    underlying: SenderImpl<T>,
    id: ChannelID,
}

impl<T: DAMType> Sender<T> {
    pub fn attach_sender(&self, sender: &dyn Context) {
        Self::log(SendEvent::AttachSender(self.id, sender.id()));
        self.underlying.attach_sender(sender);
    }

    pub fn try_send(&mut self, data: ChannelElement<T>) -> Result<(), SendOptions> {
        Self::log(SendEvent::TrySend(self.id));
        self.underlying.try_send(data)
    }

    pub fn enqueue(
        &mut self,
        manager: &mut TimeManager,
        data: ChannelElement<T>,
    ) -> Result<(), EnqueueError> {
        Self::log(SendEvent::EnqueueStart(self.id));
        let res = self.underlying.enqueue(manager, data);
        Self::log(SendEvent::EnqueueFinish(self.id));
        res
    }
}

impl<T: Clone> Cleanable for Sender<T> {
    fn cleanup(&mut self) {
        self.underlying.cleanup();
    }
}

#[log_producer]
pub struct Receiver<T: Clone> {
    underlying: ReceiverImpl<T>,
    id: ChannelID,
}

impl<T: DAMType> Receiver<T> {
    pub fn attach_receiver(&self, receiver: &dyn Context) {
        Self::log(ReceiverEvent::AttachReceiver(self.id, receiver.id()));
        self.underlying.attach_receiver(receiver)
    }

    pub fn peek(&mut self) -> Recv<T> {
        Self::log(ReceiverEvent::Peek(self.id));
        self.underlying.peek()
    }
    pub fn peek_next(&mut self, manager: &mut TimeManager) -> Recv<T> {
        Self::log(ReceiverEvent::PeekNextStart(self.id));
        let result = self.underlying.peek_next(manager);
        Self::log(ReceiverEvent::PeekNextFinish(self.id));
        result
    }

    pub fn dequeue(&mut self, manager: &mut TimeManager) -> Recv<T> {
        Self::log(ReceiverEvent::DequeueStart(self.id));
        let result = self.underlying.dequeue(manager);
        Self::log(ReceiverEvent::DequeueFinish(self.id));
        result
    }
}

impl<T: Clone> Cleanable for Receiver<T> {
    fn cleanup(&mut self) {
        self.underlying.cleanup();
    }
}

pub fn bounded<T>(capacity: usize) -> (Sender<T>, Receiver<T>)
where
    T: DAMType,
{
    bounded_with_flavor(capacity, ChannelFlavor::Unknown)
}

pub fn bounded_with_flavor<T>(capacity: usize, flavor: ChannelFlavor) -> (Sender<T>, Receiver<T>)
where
    T: DAMType,
{
    let (tx, rx) = channel::bounded::<ChannelElement<T>>(capacity);
    let (resp_t, resp_r) = channel::bounded::<Time>(capacity);
    let view_struct = Arc::new(ViewStruct::new(flavor));
    let id = ChannelID::new();
    match flavor {
        ChannelFlavor::Unknown | ChannelFlavor::Cyclic => {
            let snd = Sender {
                underlying: CyclicSender::new(tx, resp_r, capacity, view_struct.clone()).into(),
                id,
            };

            let rcv = Receiver {
                underlying: CyclicReceiver {
                    underlying: receiver::ReceiverState::Open(rx),
                    resp: resp_t,
                    view_struct,
                    head: Recv::Unknown,
                }
                .into(),
                id,
            };
            (snd, rcv)
        }
        ChannelFlavor::Acyclic => {
            let snd = Sender {
                underlying: AcyclicSender::new(tx, resp_r, capacity, view_struct.clone()).into(),
                id,
            };

            let rcv = Receiver {
                underlying: AcyclicReceiver {
                    underlying: receiver::ReceiverState::Open(rx),
                    resp: resp_t,
                    view_struct,
                    head: Recv::Unknown,
                }
                .into(),
                id,
            };
            (snd, rcv)
        }
    }
}

pub fn unbounded<T: Clone>() -> (Sender<T>, Receiver<T>)
where {
    let (tx, rx) = channel::unbounded::<ChannelElement<T>>();
    // TODO: Make dedicated unbounded senders/receivers.
    let view_struct = Arc::new(ViewStruct::new(ChannelFlavor::Unknown));
    let id = ChannelID::new();

    let snd = Sender {
        underlying: InfiniteSender::new(sender::SenderState::Open(tx), view_struct.clone()).into(),
        id,
    };

    let rcv = Receiver {
        underlying: InfiniteReceiver::new(receiver::ReceiverState::Open(rx), view_struct).into(),
        id,
    };
    (snd, rcv)
}

pub fn void<T: Clone>() -> Sender<T> {
    Sender {
        underlying: VoidSender::<T>::default().into(),
        id: ChannelID::new(),
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
