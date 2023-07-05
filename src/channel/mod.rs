pub mod utils;

use std::sync::{Arc, RwLock};

use crate::context::Context;
use crate::types::DAMType;
use crate::{context::view::*, time::Time, types::Cleanable};
use crossbeam::channel::{self, select, SendError};

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

type ViewType = Option<Box<dyn ContextView>>;

enum SenderState<T> {
    Open(channel::Sender<T>),
    Closed,
}

#[derive(Default)]
struct ViewData {
    pub sender: ViewType,
    pub receiver: ViewType,
}

#[derive(Default)]
struct ViewStruct {
    pub sender_views: RwLock<ViewData>,
    pub receiver_views: RwLock<ViewData>,
}

impl ViewStruct {
    pub fn attach_sender(&self, sender: &dyn Context) {
        self.sender_views.write().unwrap().sender = Some(sender.view());
        self.receiver_views.write().unwrap().sender = Some(sender.view());
    }

    pub fn attach_receiver(&self, receiver: &dyn Context) {
        self.sender_views.write().unwrap().receiver = Some(receiver.view());
        self.receiver_views.write().unwrap().receiver = Some(receiver.view());
    }
}

pub struct Sender<T> {
    underlying: SenderState<ChannelElement<T>>,
    resp: channel::Receiver<Time>,
    send_receive_delta: usize,
    capacity: usize,

    view_struct: Arc<ViewStruct>,
    backlog: Option<Time>,
    next_available: Option<Time>,
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

    pub fn send(&mut self, elem: ChannelElement<T>) -> Result<(), Time> {
        if self.is_full() {
            match self.next_available {
                Some(time) => return Result::Err(time),
                None => return Result::Err(self.sender_tlb() + 1),
            }
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

    fn update_len(&mut self) {
        let send_time = self.sender_tlb();
        if let Some(time) = self.backlog {
            if time < send_time {
                self.backlog = None;
                assert_ne!(self.send_receive_delta, 0);
                self.send_receive_delta -= 1;
            } else {
                self.next_available = Some(time);
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
        let mut update_srd = |time: Time, next_avail: &mut Option<Time>| {
            if send_time < time {
                *next_avail = Some(time);
                false
            } else {
                assert_ne!(self.send_receive_delta, 0);
                self.send_receive_delta -= 1;
                true
            }
        };

        loop {
            select! {
                recv(signal) -> _ => {
                    while let Ok(recv_time) = self.resp.try_recv() {
                        if !update_srd(recv_time, &mut self.next_available) {
                            return
                        }
                    }
                    self.next_available = Some(send_time + 1);
                    return
                },
                recv(self.resp) -> recv_time => {
                    if !update_srd(recv_time.unwrap(), &mut self.next_available) {
                        return
                    }
                }
            }
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
    head: Option<Recv<T>>,
}

#[derive(Clone)]
pub enum Recv<T> {
    Something(ChannelElement<T>),
    Nothing(Time),
    Closed,
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
            .sender
            .as_ref()
            .unwrap()
            .tick_lower_bound()
    }

    pub fn peek(&mut self) -> Recv<T> {
        let recv_time = self.receiver_tlb();
        match &self.head {
            Some(Recv::Nothing(time)) if *time >= recv_time => {
                return Recv::Nothing(*time);
            }
            Some(Recv::Nothing(_)) => {
                // Fallthrough, this is a stale Nothing
            }
            Some(stuff) => return stuff.clone(),
            None => {}
        }
        let update_head = |recv: &crossbeam::channel::Receiver<ChannelElement<T>>| {
            match recv.try_recv() {
                Ok(data) => Some(Recv::Something(data)),
                Err(channel::TryRecvError::Disconnected) => Some(Recv::Closed),
                Err(channel::TryRecvError::Empty) => {
                    // Fallthrough, time to do some waiting
                    None
                }
            }
        };
        if let Some(stuff) = update_head(self.under()) {
            self.head = Some(stuff.clone());
            return stuff;
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
        select! {
            recv(signal) -> send_time => {
                if let Some(stuff) = update_head(self.under()) {
                    self.head = Some(stuff.clone());
                    return stuff;
                }
                self.head = Some(Recv::Nothing(send_time.unwrap()));
            }
            recv(self.under()) -> data => {
                self.head = match data {
                    Ok(stuff) => Some(Recv::Something(stuff)),
                    Err(channel::RecvError) =>  Some(Recv::Closed),
                };
            }
        }
        self.head.clone().unwrap()
    }

    pub fn recv(&mut self) -> Recv<T> {
        let res = self.peek();
        self.head = None;
        if let Recv::Something(stuff) = &res {
            let ct: Time = self.receiver_tlb();
            let _ = self.resp.send(ct.max(stuff.time));
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
    let view_struct = Arc::new(ViewStruct::default());
    let snd = Sender {
        underlying: SenderState::Open(tx),
        resp: resp_r,
        send_receive_delta: 0,
        capacity,
        view_struct: view_struct.clone(),
        backlog: None,
        next_available: None,
    };
    let rcv = Receiver {
        underlying: ReceiverState::Open(rx),
        resp: resp_t,
        view_struct,
        head: None,
    };
    (snd, rcv)
}

pub fn unbounded<T>() -> (Sender<T>, Receiver<T>)
where
    T: DAMType,
{
    let (tx, rx) = channel::unbounded::<ChannelElement<T>>();
    let (resp_t, resp_r) = channel::unbounded::<Time>();
    let view_struct = Arc::new(ViewStruct::default());
    let snd = Sender {
        underlying: SenderState::Open(tx),
        resp: resp_r,
        send_receive_delta: 0,
        capacity: usize::MAX,
        view_struct: view_struct.clone(),
        backlog: None,
        next_available: None,
    };
    let rcv = Receiver {
        underlying: ReceiverState::Open(rx),
        resp: resp_t,
        view_struct,
        head: None,
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
