use std::sync::Arc;

use crate::{context::ContextView, time::Time};
use crossbeam::channel::{self, select};

type ContextViewRef<T> = Arc<T>;

#[derive(Clone, Copy)]
pub struct ChannelElement<T> {
    time: Time,
    data: T,
}

pub struct Sender<T, S, R> {
    underlying: channel::Sender<ChannelElement<T>>,
    resp: channel::Receiver<Time>,
    send_receive_delta: usize,
    capacity: usize,

    sender: ContextViewRef<S>,
    receiver: ContextViewRef<R>,
    backlog: Option<Time>,
    next_available: Option<Time>,
}

impl<T: Copy, S: ContextView, R: ContextView> Sender<T, S, R> {
    pub fn send(&mut self, elem: ChannelElement<T>) -> Result<(), Time> {
        if self.is_full() {
            match self.next_available {
                Some(time) => return Result::Err(time),
                None => return Result::Err(self.sender.tick_lower_bound() + 1),
            }
        }

        assert!(self.send_receive_delta < self.capacity);
        self.underlying.send(elem).unwrap();
        self.send_receive_delta += 1;
        Ok(())
    }

    fn is_full(&mut self) -> bool {
        if self.send_receive_delta < self.capacity {
            return false;
        }
        self.update_len();

        return self.send_receive_delta < self.capacity;
    }

    fn update_len(&mut self) {
        let send_time = self.sender.tick_lower_bound();
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

        let signal = self.receiver.signal_when(self.sender.tick_lower_bound());
        let mut update_srd = |time: Time| {
            if send_time < time {
                self.next_available = Some(time);
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
                        if !update_srd(recv_time) {
                            return
                        }
                    }
                },
                recv(self.resp) -> recv_time => {
                    if !update_srd(recv_time.unwrap()) {
                        return
                    }
                }
            }
        }
    }
}

pub struct Receiver<T, S, R> {
    underlying: channel::Receiver<ChannelElement<T>>,
    resp: channel::Sender<Time>,

    sender: ContextViewRef<S>,
    receiver: ContextViewRef<R>,
    head: Option<Recv<T>>,
}

#[derive(Clone, Copy)]
pub enum Recv<T> {
    Something(ChannelElement<T>),
    Nothing(Time),
    Closed,
}

impl<T: Copy, S: ContextView, R: ContextView> Receiver<T, S, R> {
    pub fn peek(&mut self) -> Recv<T> {
        let recv_time = self.receiver.tick_lower_bound();
        match self.head {
            Some(Recv::Nothing(time)) if time >= recv_time => {
                return Recv::Nothing(time);
            }
            Some(Recv::Nothing(_)) => {
                // Fallthrough, this is a stale Nothing
            }
            Some(stuff) => return stuff,
            None => {}
        }
        let update_head = || {
            match self.underlying.try_recv() {
                Ok(data) => Some(Recv::Something(data)),
                Err(channel::TryRecvError::Disconnected) => Some(Recv::Closed),
                Err(channel::TryRecvError::Empty) => {
                    // Fallthrough, time to do some waiting
                    None
                }
            }
        };
        if let Some(stuff) = update_head() {
            self.head = Some(stuff);
            return stuff;
        }

        let signal = self.sender.signal_when(recv_time);
        select! {
            recv(signal) -> send_time => {
                if let Some(stuff) = update_head() {
                    self.head = Some(stuff);
                    return stuff;
                }
                self.head = Some(Recv::Nothing(send_time.unwrap()));
            }
            recv(self.underlying) -> data => {
                self.head = match data {
                    Ok(stuff) => Some(Recv::Something(stuff)),
                    Err(channel::RecvError) =>  Some(Recv::Closed),
                };
            }
        }
        self.head.unwrap()
    }
    pub fn recv(&mut self) -> Recv<T> {
        unimplemented!()
    }
}

pub fn bounded<T, S, R>(
    capacity: usize,
    sender: ContextViewRef<S>,
    receiver: ContextViewRef<R>,
) -> (Sender<T, S, R>, Receiver<T, S, R>)
where
    T: Copy,
    S: ContextView,
    R: ContextView,
{
    let (tx, rx) = channel::bounded::<ChannelElement<T>>(capacity);
    let (resp_t, resp_r) = channel::bounded::<Time>(capacity);
    let snd = Sender {
        underlying: tx,
        resp: resp_r,
        send_receive_delta: 0,
        capacity,
        sender: sender.clone(),
        receiver: receiver.clone(),
        backlog: None,
        next_available: None,
    };
    let rcv = Receiver {
        underlying: rx,
        resp: resp_t,
        sender: sender.clone(),
        receiver: receiver.clone(),
        head: None,
    };
    (snd, rcv)
}
