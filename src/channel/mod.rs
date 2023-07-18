mod channel_id;
pub mod utils;
pub use channel_id::*;

mod events;
use events::*;

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

use dam_core::time::Time;
use dam_macros::log_producer;

use self::receiver::AcyclicReceiver;
use self::receiver::CyclicReceiver;
use self::receiver::{ReceiverFlavor, ReceiverImpl};
use self::sender::CyclicSender;
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
}

impl<T: DAMType> Sender<T> {
    pub fn attach_sender(&self, sender: &dyn Context) {
        self.underlying.attach_sender(sender);
    }

    pub fn try_send(&mut self, data: ChannelElement<T>) -> Result<(), SendOptions> {
        self.underlying.try_send(data)
    }

    pub fn enqueue(
        &mut self,
        manager: &mut TimeManager,
        data: ChannelElement<T>,
    ) -> Result<(), EnqueueError> {
        self.underlying.enqueue(manager, data)
    }
}

impl<T: Clone> Cleanable for Sender<T> {
    fn cleanup(&mut self) {}
}

pub struct Receiver<T: Clone> {
    underlying: ReceiverImpl<T>,
}

impl<T: DAMType> Receiver<T> {
    pub fn attach_receiver(&self, receiver: &dyn Context) {
        self.underlying.attach_receiver(receiver)
    }

    pub fn peek(&mut self) -> Recv<T> {
        self.underlying.peek()
    }
    pub fn peek_next(&mut self, manager: &mut TimeManager) -> Recv<T> {
        self.underlying.peek_next(manager)
    }

    pub fn dequeue(&mut self, manager: &mut TimeManager) -> Recv<T> {
        self.underlying.dequeue(manager)
    }
}

impl<T: Clone> Cleanable for Receiver<T> {
    fn cleanup(&mut self) {}
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
    match flavor {
        ChannelFlavor::Unknown | ChannelFlavor::Cyclic => {
            let snd = Sender {
                underlying: CyclicSender {
                    underlying: sender::SenderState::Open(tx),
                    resp: resp_r,
                    send_receive_delta: 0,
                    capacity,
                    view_struct: view_struct.clone(),
                    next_available: SendOptions::Unknown,
                }
                .into(),
            };

            let rcv = Receiver {
                underlying: CyclicReceiver {
                    underlying: receiver::ReceiverState::Open(rx),
                    resp: resp_t,
                    view_struct,
                    head: Recv::Unknown,
                }
                .into(),
            };
            (snd, rcv)
        }
        ChannelFlavor::Acyclic => {
            let snd = Sender {
                underlying: CyclicSender {
                    underlying: sender::SenderState::Open(tx),
                    resp: resp_r,
                    send_receive_delta: 0,
                    capacity,
                    view_struct: view_struct.clone(),
                    next_available: SendOptions::Unknown,
                }
                .into(),
            };

            let rcv = Receiver {
                underlying: AcyclicReceiver {
                    underlying: receiver::ReceiverState::Open(rx),
                    resp: resp_t,
                    view_struct,
                    head: Recv::Unknown,
                }
                .into(),
            };
            (snd, rcv)
        }
    }
}

pub fn unbounded<T: Clone>() -> (Sender<T>, Receiver<T>) {
    unbounded_with_flavor(ChannelFlavor::Unknown)
}

pub fn unbounded_with_flavor<T: Clone>(flavor: ChannelFlavor) -> (Sender<T>, Receiver<T>)
where {
    let (tx, rx) = channel::unbounded::<ChannelElement<T>>();
    let (resp_t, resp_r) = channel::unbounded::<Time>();
    let view_struct = Arc::new(ViewStruct::new(flavor));
    match flavor {
        ChannelFlavor::Unknown | ChannelFlavor::Cyclic => {
            let snd = Sender {
                underlying: CyclicSender {
                    underlying: sender::SenderState::Open(tx),
                    resp: resp_r,
                    send_receive_delta: 0,
                    capacity: usize::MAX,
                    view_struct: view_struct.clone(),
                    next_available: SendOptions::Unknown,
                }
                .into(),
            };

            let rcv = Receiver {
                underlying: CyclicReceiver {
                    underlying: receiver::ReceiverState::Open(rx),
                    resp: resp_t,
                    view_struct,
                    head: Recv::Unknown,
                }
                .into(),
            };
            (snd, rcv)
        }
        ChannelFlavor::Acyclic => {
            let snd = Sender {
                underlying: CyclicSender {
                    underlying: sender::SenderState::Open(tx),
                    resp: resp_r,
                    send_receive_delta: 0,
                    capacity: usize::MAX,
                    view_struct: view_struct.clone(),
                    next_available: SendOptions::Unknown,
                }
                .into(),
            };

            let rcv = Receiver {
                underlying: AcyclicReceiver {
                    underlying: receiver::ReceiverState::Open(rx),
                    resp: resp_t,
                    view_struct,
                    head: Recv::Unknown,
                }
                .into(),
            };
            (snd, rcv)
        }
    }
}

pub fn void<T: Clone>() -> Sender<T> {
    Sender {
        underlying: VoidSender::<T>::default().into(),
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
