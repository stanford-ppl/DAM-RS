pub mod utils;

use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, RwLock};

use crate::context::Context;
use crate::types::DAMType;
use crate::{context::view::*, time::Time, types::Cleanable};
use crossbeam::channel::{self, SendError};

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

struct ViewStruct {
    pub sender_views: RwLock<ViewData>,
    pub receiver_views: RwLock<ViewData>,

    // Unlike the other SRD, this one reflects the actual state of the channel.
    pub real_send_receive_delta: AtomicUsize,
    pub channel_id: usize,
}

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
impl ViewStruct {
    fn next_id() -> usize {
        ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn new() -> Self {
        Self {
            sender_views: Default::default(),
            receiver_views: Default::default(),
            real_send_receive_delta: Default::default(),
            channel_id: ViewStruct::next_id(),
        }
    }

    pub fn attach_sender(&self, sender: &dyn Context) {
        self.sender_views.write().unwrap().sender = Some(sender.view());
        self.receiver_views.write().unwrap().sender = Some(sender.view());
    }

    pub fn attach_receiver(&self, receiver: &dyn Context) {
        self.sender_views.write().unwrap().receiver = Some(receiver.view());
        self.receiver_views.write().unwrap().receiver = Some(receiver.view());
    }

    pub fn register_send(&self) {
        self.real_send_receive_delta
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    }

    pub fn register_recv(&self) {
        let old = self
            .real_send_receive_delta
            .fetch_sub(1, std::sync::atomic::Ordering::AcqRel);

        // If we decremented an empty channel
        assert!(old > 0);
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
            .sender_views
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
        self.view_struct.register_send();
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
        let real_srd = self
            .view_struct
            .real_send_receive_delta
            .load(std::sync::atomic::Ordering::Acquire);

        // The real_srd is decremented as soon as a read is processed, so it should be
        // strictly lower.
        assert!(self.send_receive_delta >= real_srd);
        for _ in 0..(self.send_receive_delta - real_srd) {
            // Loop until either:
            // 1. These two agree on how many values are currently in the channel
            //    In which case the sender is viewing "reality"
            // 2. The next read is in the future
            match self.resp.recv() {
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
                Err(_) => {
                    // The receiver is done reading. At this point we're done.
                    self.next_available = SendOptions::Never;
                    return;
                }
            }
        }
    }

    fn update_len(&mut self) {
        let send_time = self.sender_tlb();
        if let SendOptions::AvailableAt(time) = self.next_available {
            if time < send_time {
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

        let signal = self
            .view_struct
            .sender_views
            .read()
            .unwrap()
            .receiver
            .as_ref()
            .unwrap()
            .signal_when(send_time);

        // Wait for producer to catch up
        let new_time = signal.recv().unwrap();

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
            .receiver_views
            .read()
            .unwrap()
            .receiver
            .as_ref()
            .unwrap()
            .tick_lower_bound()
    }

    fn try_update_head(&mut self) -> bool {
        let srd = self
            .view_struct
            .real_send_receive_delta
            .load(std::sync::atomic::Ordering::Acquire);
        if srd > 0 {
            self.head = match self.under().recv() {
                Ok(data) => Recv::Something(data),
                Err(_) => Recv::Closed,
            };
            return true;
        }
        return false;
    }

    pub fn peek(&mut self) -> Recv<T> {
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
        if self.try_update_head() {
            return self.head.clone();
        }

        let signal = self
            .view_struct
            .receiver_views
            .read()
            .unwrap()
            .sender
            .as_ref()
            .unwrap()
            .signal_when(recv_time);
        let sig_time = signal.recv().unwrap();
        assert!(sig_time >= recv_time);
        if self.try_update_head() {
            return self.head.clone();
        }
        if sig_time.is_infinite() {
            match &self.head {
                Recv::Something(_) => {}
                _ => {
                    self.head = Recv::Closed;
                    return Recv::Closed;
                }
            }
        }
        Recv::Nothing(sig_time)
    }

    pub fn recv(&mut self) -> Recv<T> {
        let res = self.peek();
        match &res {
            Recv::Something(stuff) => {
                let ct: Time = self.receiver_tlb();
                let _ = self.resp.send(ct.max(stuff.time));
                self.view_struct.register_recv();
                self.head = Recv::Unknown;
            }
            Recv::Nothing(_) | Recv::Closed => {}
            Recv::Unknown => unreachable!(),
        }
        res
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
