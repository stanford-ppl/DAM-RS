pub mod utils;

use std::sync::atomic::{AtomicUsize};
use std::sync::{Arc, RwLock};

use crate::context::Context;
use crate::types::Cleanable;
use crate::types::DAMType;
use crossbeam::channel::{self, RecvError, SendError};
use dam_core::*;

use dam_core::time::Time;

#[derive(Clone, Debug)]
pub struct ChannelElement<T> {
    pub time: Time,
    pub data: T,
}

impl<T: DAMType> ChannelElement<T> {
    pub fn new(time: Time, data: T) -> ChannelElement<T> {
        ChannelElement { time, data }
    }

    pub fn update_time(&mut self, new_time: Time) {
        self.time = std::cmp::max(self.time, new_time);
    }
}

type ViewType = Option<TimeView>;

enum SenderState<T> {
    Open(channel::Sender<T>),
    Closed,
}

#[derive(Default)]
struct ViewData {
    pub sender: ViewType,
    pub receiver: ViewType,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ChannelFlavor {
    Acyclic,
    Cyclic,
    Unknown,
}

struct ViewStruct {
    pub views: RwLock<ViewData>,

    pub flavor: RwLock<ChannelFlavor>,

    pub channel_id: usize,
}

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
impl ViewStruct {
    fn next_id() -> usize {
        ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn new() -> Self {
        Self {
            views: Default::default(),
            channel_id: ViewStruct::next_id(),
            flavor: RwLock::new(ChannelFlavor::Unknown),
        }
    }

    pub fn attach_sender(&self, sender: &dyn Context) {
        self.views.write().unwrap().sender = Some(sender.view());
    }

    pub fn attach_receiver(&self, receiver: &dyn Context) {
        self.views.write().unwrap().receiver = Some(receiver.view());
    }

    pub fn set_flavor(&self, flavor: ChannelFlavor) {
        let mut write_lock = self.flavor.write().unwrap();
        match *write_lock {
            ChannelFlavor::Unknown => *write_lock = flavor,
            old_flavor if old_flavor == flavor => {}
            _ => {
                // Wasn't unkown, and the flavors didn't match
                panic!(
                    "Attempted to set conflicting flavors: Old ({:?}) != New ({:?})",
                    *write_lock, flavor
                );
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SendOptions {
    Unknown,
    AvailableAt(Time),
    CheckBackAt(Time),
    Never,
}
pub struct Sender<T> {
    underlying: SenderState<ChannelElement<T>>,
    resp: channel::Receiver<Time>,
    send_receive_delta: usize,
    capacity: usize,

    view_struct: Arc<ViewStruct>,
    next_available: SendOptions,
}

impl<T: DAMType> Sender<T> {
    fn under_send(&mut self, elem: ChannelElement<T>) -> Result<(), SendError<ChannelElement<T>>> {
        match &self.underlying {
            SenderState::Open(sender) => sender.send(elem),
            SenderState::Closed => Err(SendError(elem)),
        }
    }

    fn sender_tlb(&self) -> Time {
        self.view_struct
            .views
            .read()
            .unwrap()
            .sender
            .as_ref()
            .unwrap()
            .tick_lower_bound()
    }

    pub fn send(&mut self, elem: ChannelElement<T>) -> Result<(), SendOptions> {
        if self.is_full() {
            return Err(self.next_available);
        }

        assert!(self.send_receive_delta < self.capacity);
        assert!(elem.time >= self.sender_tlb());
        self.under_send(elem).unwrap();
        self.send_receive_delta += 1;
        Ok(())
    }

    pub fn attach_sender(&self, sender: &dyn Context) {
        self.view_struct.attach_sender(sender);
    }

    fn is_full(&mut self) -> bool {
        if self.send_receive_delta < self.capacity {
            return false;
        }
        self.update_len();

        self.send_receive_delta == self.capacity
    }

    fn update_srd(&mut self) {
        let send_time = self.sender_tlb();
        // We don't know when it'll be available.
        self.next_available = SendOptions::Unknown;

        loop {
            match self.resp.try_recv() {
                Ok(time) if time <= send_time => {
                    assert!(self.send_receive_delta > 0);
                    self.send_receive_delta -= 1;
                }
                Ok(time) => {
                    // Got a time in the future
                    assert!(self.next_available == SendOptions::Unknown);
                    self.next_available = SendOptions::AvailableAt(time);
                    return;
                }
                Err(channel::TryRecvError::Disconnected) => {
                    self.next_available = SendOptions::Never;
                    return;
                }
                Err(channel::TryRecvError::Empty) => {
                    return;
                }
            }
        }
    }

    fn update_len(&mut self) {
        let send_time = self.sender_tlb();
        if let SendOptions::AvailableAt(time) = self.next_available {
            if time <= send_time {
                // Next available time has already passed, so we pop an element off.
                // Additionally, to avoid work, we don't update next_available immediately.
                self.next_available = SendOptions::Unknown;
                assert_ne!(self.send_receive_delta, 0);
                self.send_receive_delta -= 1;
            } else {
                // Next available time in the future, becomes a no-op.
                return;
            }
        }

        let new_time = self
            .view_struct
            .views
            .read()
            .unwrap()
            .receiver
            .as_ref()
            .unwrap()
            .wait_until(send_time);

        // Forces the resp channel to synchronize w.r.t. the signal.

        self.update_srd();
        if self.next_available == SendOptions::Unknown {
            self.next_available = SendOptions::CheckBackAt(new_time + 1)
        }
    }
}

impl<T> Cleanable for Sender<T> {
    fn cleanup(&mut self) {
        self.close();
    }
}

impl<T> Sender<T> {
    // This drops the underlying channel
    pub fn close(&mut self) {
        self.underlying = SenderState::Closed;
    }

    pub fn set_flavor(&self, flavor: ChannelFlavor) {
        self.view_struct.set_flavor(flavor);
    }
}

enum ReceiverState<T> {
    Open(channel::Receiver<T>),
    Closed,
}

pub struct Receiver<T> {
    underlying: ReceiverState<ChannelElement<T>>,
    resp: channel::Sender<Time>,

    view_struct: Arc<ViewStruct>,
    head: Recv<T>,
}

#[derive(Clone, Debug)]
pub enum Recv<T> {
    Something(ChannelElement<T>),
    Nothing(Time),
    Closed,
    Unknown,
}

impl<T: DAMType> Receiver<T> {
    fn under(&mut self) -> &crossbeam::channel::Receiver<ChannelElement<T>> {
        match &self.underlying {
            ReceiverState::Open(chan) => chan,
            ReceiverState::Closed => panic!("Attempting to read from a closed channel!"),
        }
    }

    fn receiver_tlb(&self) -> Time {
        self.view_struct
            .views
            .read()
            .unwrap()
            .receiver
            .as_ref()
            .unwrap()
            .tick_lower_bound()
    }

    fn try_update_head(&mut self, nothing_time: Time) -> bool {
        let mut retflag = false;
        self.head = match self.under().try_recv() {
            Ok(data) => {
                retflag = true;
                Recv::Something(data)
            }
            Err(channel::TryRecvError::Disconnected) => {
                retflag = true;
                Recv::Closed
            }
            Err(channel::TryRecvError::Empty) if nothing_time.is_infinite() => {
                retflag = true;
                Recv::Closed
            }
            Err(channel::TryRecvError::Empty) => Recv::Nothing(nothing_time),
        };
        return retflag;
    }

    pub fn peek(&mut self) -> Recv<T> {
        let flavor: ChannelFlavor = *self.view_struct.flavor.read().unwrap();
        match flavor {
            ChannelFlavor::Acyclic => self.peek_sync(),
            ChannelFlavor::Cyclic => self.peek_async(),
            ChannelFlavor::Unknown => self.peek_async(),
        }
    }

    // This one blocks, so it could cause deadlock!
    fn peek_sync(&mut self) -> Recv<T> {
        match self.head {
            Recv::Something(_) => return self.head.clone(),
            Recv::Closed => return Recv::Closed,
            _ => {}
        }

        // Otherwise, we recv from the channel.
        self.head = match self.under().recv() {
            Ok(value) => Recv::Something(value),
            Err(RecvError) => Recv::Closed,
        };

        self.head.clone()
    }

    fn peek_async(&mut self) -> Recv<T> {
        let recv_time = self.receiver_tlb();
        match self.head {
            Recv::Nothing(time) if time >= recv_time => {
                // This is a valid nothing
                return Recv::Nothing(time);
            }
            Recv::Nothing(_) | Recv::Unknown => {}
            Recv::Something(_) => return self.head.clone(),
            Recv::Closed => return Recv::Closed,
        }

        // First attempt, it's ok if we get nothing.
        if self.try_update_head(Time::new(0)) {
            return self.head.clone();
        }

        let sig_time = self
            .view_struct
            .views
            .read()
            .unwrap()
            .sender
            .as_ref()
            .unwrap()
            .wait_until(recv_time);
        assert!(sig_time >= recv_time);
        self.try_update_head(sig_time);
        return self.head.clone();
    }

    pub fn recv(&mut self) -> Recv<T> {
        let flavor: ChannelFlavor = *self.view_struct.flavor.read().unwrap();
        match flavor {
            ChannelFlavor::Acyclic => self.recv_sync(),
            ChannelFlavor::Cyclic => self.recv_async(),
            ChannelFlavor::Unknown => self.recv_async(),
        }
    }

    fn recv_async(&mut self) -> Recv<T> {
        let res = self.peek();
        match &res {
            Recv::Something(stuff) => {
                let ct: Time = self.receiver_tlb();
                let _ = self.resp.send(ct.max(stuff.time));
                self.head = Recv::Unknown;
            }
            Recv::Nothing(_) | Recv::Closed => {}
            Recv::Unknown => unreachable!(),
        }
        res
    }

    fn recv_sync(&mut self) -> Recv<T> {
        let res = self.peek_sync();
        match &res {
            Recv::Something(result) => {
                let ct: Time = self.receiver_tlb();
                let _ = self.resp.send(ct.max(result.time));
                self.head = Recv::Unknown;
                return res.clone();
            }
            Recv::Closed => return res.clone(),
            _ => unreachable!(),
        }
    }

    pub fn attach_receiver(&self, receiver: &dyn Context) {
        self.view_struct.attach_receiver(receiver);
    }
}

impl<T> Receiver<T> {
    // This drops the underlying channel
    pub fn close(&mut self) {
        self.underlying = ReceiverState::Closed;
    }

    pub fn set_flavor(&self, flavor: ChannelFlavor) {
        self.view_struct.set_flavor(flavor);
    }
}

impl<T> Cleanable for Receiver<T> {
    fn cleanup(&mut self) {
        self.close();
    }
}

pub fn bounded<T>(capacity: usize) -> (Sender<T>, Receiver<T>)
where
    T: DAMType,
{
    let (tx, rx) = channel::bounded::<ChannelElement<T>>(capacity);
    let (resp_t, resp_r) = channel::bounded::<Time>(capacity);
    let view_struct = Arc::new(ViewStruct::new());
    let snd = Sender {
        underlying: SenderState::Open(tx),
        resp: resp_r,
        send_receive_delta: 0,
        capacity,
        view_struct: view_struct.clone(),
        next_available: SendOptions::Unknown,
    };
    let rcv = Receiver {
        underlying: ReceiverState::Open(rx),
        resp: resp_t,
        view_struct,
        head: Recv::Unknown,
    };
    (snd, rcv)
}

pub fn unbounded<T>() -> (Sender<T>, Receiver<T>)
where
    T: DAMType,
{
    let (tx, rx) = channel::unbounded::<ChannelElement<T>>();
    let (resp_t, resp_r) = channel::unbounded::<Time>();
    let view_struct = Arc::new(ViewStruct::new());
    let snd = Sender {
        underlying: SenderState::Open(tx),
        resp: resp_r,
        send_receive_delta: 0,
        capacity: usize::MAX,
        view_struct: view_struct.clone(),
        next_available: SendOptions::Unknown,
    };
    let rcv = Receiver {
        underlying: ReceiverState::Open(rx),
        resp: resp_t,
        view_struct,
        head: Recv::Unknown,
    };
    (snd, rcv)
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
