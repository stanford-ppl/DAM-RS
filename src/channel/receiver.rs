use std::{marker::PhantomData, sync::Arc};

use crossbeam::channel::{self, RecvError, TryRecvError};

use dam_core::{time::Time, TimeManager};

use enum_dispatch::enum_dispatch;

use crate::context::Context;

use super::{channel_spec::ChannelSpec, ChannelElement, Recv};

pub(crate) enum ReceiverState<T> {
    Open(channel::Receiver<T>),
    Closed,
}

#[enum_dispatch(ReceiverImpl<T>)]
pub(crate) trait ReceiverFlavor<T> {
    fn attach_receiver(&self, receiver: &dyn Context);
    fn peek(&mut self) -> Recv<T>;
    fn peek_next(&mut self, manager: &mut TimeManager) -> Recv<T>;
    fn dequeue(&mut self, manager: &mut TimeManager) -> Recv<T>;
    fn cleanup(&mut self);
}

#[enum_dispatch]
pub(crate) enum ReceiverImpl<T: Clone> {
    Cyclic(CyclicReceiver<T>),
    Acyclic(AcyclicReceiver<T>),
    AcyclicInfinite(AcyclicInfiniteReceiver<T>),
    CyclicInfinite(CyclicInfiniteReceiver<T>),
    Undefined(UndefinedReceiver<T>),
}

pub struct UndefinedReceiver<T> {
    spec: Arc<ChannelSpec>,
    _marker: PhantomData<T>,
}

impl<T> UndefinedReceiver<T> {
    pub fn new(spec: Arc<ChannelSpec>) -> Self {
        Self {
            spec,
            _marker: PhantomData,
        }
    }
}

impl<T> ReceiverFlavor<T> for UndefinedReceiver<T> {
    fn attach_receiver(&self, receiver: &dyn Context) {
        self.spec.attach_receiver(receiver);
    }

    fn peek(&mut self) -> Recv<T> {
        panic!();
    }

    fn peek_next(&mut self, _manager: &mut TimeManager) -> Recv<T> {
        panic!();
    }

    fn dequeue(&mut self, _manager: &mut TimeManager) -> Recv<T> {
        panic!();
    }

    fn cleanup(&mut self) {
        // No-op since it's part of drop
    }
}

pub(crate) struct CyclicReceiver<T> {
    pub(crate) underlying: ReceiverState<ChannelElement<T>>,
    pub(crate) resp: channel::Sender<Time>,

    pub(crate) view_struct: Arc<ChannelSpec>,
    pub(crate) head: Recv<T>,
}

impl<T: Clone> ReceiverFlavor<T> for CyclicReceiver<T> {
    fn attach_receiver(&self, receiver: &dyn Context) {
        self.view_struct.attach_receiver(receiver);
    }

    fn peek(&mut self) -> Recv<T> {
        let recv_time = self.view_struct.receiver_tlb();
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

        let sig_time = self.view_struct.wait_until_sender(recv_time);
        assert!(sig_time >= recv_time);
        self.try_update_head(sig_time);
        self.head.clone()
    }

    fn peek_next(&mut self, manager: &mut TimeManager) -> Recv<T> {
        loop {
            let v: Recv<T> = self.peek();
            match v {
                Recv::Nothing(time) => manager.advance(time + 1), // Nothing here, so tick forward until there might be
                Recv::Closed => return Recv::Closed, // Channel is closed, so let the dequeuer know
                Recv::Something(stuff) => {
                    manager.advance(stuff.time);
                    return Recv::Something(stuff);
                }
                Recv::Unknown => panic!("Can't peek_next an unknown!"),
            }
        }
    }

    fn dequeue(&mut self, manager: &mut TimeManager) -> Recv<T> {
        loop {
            let v = self.recv();
            match v {
                Recv::Nothing(time) => manager.advance(time + 1), // Nothing here, so tick forward until there might be
                Recv::Closed => return Recv::Closed, // Channel is closed, so let the dequeuer know
                Recv::Something(stuff) => {
                    manager.advance(stuff.time);
                    return Recv::Something(stuff);
                }
                Recv::Unknown => panic!("Can't receive an Unknown!"),
            }
        }
    }

    fn cleanup(&mut self) {
        self.underlying = ReceiverState::Closed;
    }
}

impl<T: Clone> CyclicReceiver<T> {
    pub fn new(
        underlying: channel::Receiver<ChannelElement<T>>,
        resp: channel::Sender<Time>,
        view_struct: Arc<ChannelSpec>,
    ) -> Self {
        Self {
            underlying: ReceiverState::Open(underlying),
            resp,
            view_struct,
            head: Recv::Unknown,
        }
    }
    fn recv(&mut self) -> Recv<T> {
        let res = self.peek();
        match &res {
            Recv::Something(stuff) => {
                self.register_recv(stuff.time);
                self.head = Recv::Unknown;
            }
            Recv::Nothing(_) | Recv::Closed => {}
            Recv::Unknown => panic!("We shouldn't be receiving unknowns!"),
        }
        res
    }

    fn try_recv(&mut self) -> Result<ChannelElement<T>, TryRecvError> {
        match &self.underlying {
            ReceiverState::Open(chan) => chan.try_recv(),
            ReceiverState::Closed => Err(TryRecvError::Disconnected),
        }
    }

    fn try_update_head(&mut self, nothing_time: Time) -> bool {
        let mut retflag = false;
        self.head = match self.try_recv() {
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
        retflag
    }

    fn register_recv(&mut self, time: Time) {
        let ct: Time = self.view_struct.receiver_tlb();
        let prev_srd = self.view_struct.register_recv();
        let _ = self.resp.send(ct.max(time));
        assert_ne!(prev_srd, 0);
    }
}

pub struct AcyclicReceiver<T> {
    pub(crate) underlying: ReceiverState<ChannelElement<T>>,
    pub(crate) resp: channel::Sender<Time>,

    pub(crate) view_struct: Arc<ChannelSpec>,
    pub(crate) head: Recv<T>,
}

impl<T: Clone> ReceiverFlavor<T> for AcyclicReceiver<T> {
    fn attach_receiver(&self, receiver: &dyn Context) {
        self.view_struct.attach_receiver(receiver);
    }

    fn peek(&mut self) -> Recv<T> {
        let recv_time = self.view_struct.receiver_tlb();
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

        let sig_time = self.view_struct.wait_until_sender(recv_time);
        assert!(sig_time >= recv_time);
        self.try_update_head(sig_time);
        self.head.clone()
    }
    fn peek_next(&mut self, manager: &mut TimeManager) -> Recv<T> {
        match &self.head {
            Recv::Something(ce) => {
                manager.advance(ce.time);
                return self.head.clone();
            }
            Recv::Nothing(_) | Recv::Unknown => {}
            Recv::Closed => return Recv::Closed,
        }

        self.head = match self.under_recv() {
            Ok(stuff) => {
                manager.advance(stuff.time);
                Recv::Something(stuff)
            }
            Err(_) => Recv::Closed,
        };

        self.head.clone()
    }

    fn dequeue(&mut self, manager: &mut TimeManager) -> Recv<T> {
        if let Recv::Something(x) = &self.head {
            let time = x.time;
            let mut result = Recv::Unknown;
            std::mem::swap(&mut self.head, &mut result);
            self.register_recv(time);
            manager.advance(time);
            return result;
        }

        if let Recv::Closed = self.head {
            return Recv::Closed;
        }

        // At this point, we can just block!
        match self.under_recv() {
            Ok(ce) => {
                self.register_recv(ce.time);
                manager.advance(ce.time);
                Recv::Something(ce)
            }
            Err(_) => {
                self.head = Recv::Closed;
                Recv::Closed
            }
        }
    }

    fn cleanup(&mut self) {
        self.underlying = ReceiverState::Closed;
    }
}

impl<T: Clone> AcyclicReceiver<T> {
    pub fn new(
        underlying: channel::Receiver<ChannelElement<T>>,
        resp: channel::Sender<Time>,
        view_struct: Arc<ChannelSpec>,
    ) -> Self {
        Self {
            underlying: ReceiverState::Open(underlying),
            resp,
            view_struct,
            head: Recv::Unknown,
        }
    }

    fn try_update_head(&mut self, nothing_time: Time) -> bool {
        let mut retflag = false;
        self.head = match self.try_recv() {
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
        retflag
    }

    fn try_recv(&mut self) -> Result<ChannelElement<T>, TryRecvError> {
        match &self.underlying {
            ReceiverState::Open(chan) => chan.try_recv(),
            ReceiverState::Closed => Err(TryRecvError::Disconnected),
        }
    }

    fn under_recv(&mut self) -> Result<ChannelElement<T>, RecvError> {
        match &self.underlying {
            ReceiverState::Open(chan) => chan.recv(),
            ReceiverState::Closed => Err(RecvError),
        }
    }

    fn register_recv(&mut self, time: Time) {
        let ct: Time = self.view_struct.receiver_tlb();
        let prev_srd = self.view_struct.register_recv();
        let _ = self.resp.send(ct.max(time));
        assert_ne!(prev_srd, 0);
    }
}

pub struct AcyclicInfiniteReceiver<T> {
    underlying: ReceiverState<ChannelElement<T>>,

    view_struct: Arc<ChannelSpec>,
    head: Recv<T>,
}

impl<T: Clone> ReceiverFlavor<T> for AcyclicInfiniteReceiver<T> {
    fn attach_receiver(&self, receiver: &dyn Context) {
        self.view_struct.attach_receiver(receiver);
    }

    fn peek(&mut self) -> Recv<T> {
        let recv_time = self.view_struct.receiver_tlb();
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

        let sig_time = self.view_struct.wait_until_sender(recv_time);
        assert!(sig_time >= recv_time);
        self.try_update_head(sig_time);
        self.head.clone()
    }
    fn peek_next(&mut self, manager: &mut TimeManager) -> Recv<T> {
        match &self.head {
            Recv::Something(ce) => {
                manager.advance(ce.time);
                return self.head.clone();
            }
            Recv::Nothing(_) | Recv::Unknown => {}
            Recv::Closed => return Recv::Closed,
        }

        self.head = match self.under_recv() {
            Ok(stuff) => {
                manager.advance(stuff.time);
                Recv::Something(stuff)
            }
            Err(_) => Recv::Closed,
        };

        self.head.clone()
    }

    fn dequeue(&mut self, manager: &mut TimeManager) -> Recv<T> {
        if let Recv::Something(x) = &self.head {
            let time = x.time;
            let mut result = Recv::Unknown;
            std::mem::swap(&mut self.head, &mut result);
            self.view_struct.register_recv();
            manager.advance(time);
            return result;
        }

        if let Recv::Closed = self.head {
            return Recv::Closed;
        }

        // At this point, we can just block!
        match self.under_recv() {
            Ok(ce) => {
                self.view_struct.register_recv();
                manager.advance(ce.time);
                Recv::Something(ce)
            }
            Err(_) => {
                self.head = Recv::Closed;
                Recv::Closed
            }
        }
    }

    fn cleanup(&mut self) {
        self.underlying = ReceiverState::Closed;
    }
}

impl<T: Clone> AcyclicInfiniteReceiver<T> {
    pub(crate) fn new(
        underlying: ReceiverState<ChannelElement<T>>,
        view_struct: Arc<ChannelSpec>,
    ) -> Self {
        Self {
            underlying,
            view_struct,
            head: Recv::Unknown,
        }
    }

    fn try_update_head(&mut self, nothing_time: Time) -> bool {
        let mut retflag = false;
        self.head = match self.try_recv() {
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
        retflag
    }

    fn try_recv(&mut self) -> Result<ChannelElement<T>, TryRecvError> {
        match &self.underlying {
            ReceiverState::Open(chan) => chan.try_recv(),
            ReceiverState::Closed => Err(TryRecvError::Disconnected),
        }
    }

    fn under_recv(&mut self) -> Result<ChannelElement<T>, RecvError> {
        match &self.underlying {
            ReceiverState::Open(chan) => chan.recv(),
            ReceiverState::Closed => Err(RecvError),
        }
    }
}

pub struct CyclicInfiniteReceiver<T> {
    underlying: ReceiverState<ChannelElement<T>>,

    view_struct: Arc<ChannelSpec>,
    head: Recv<T>,
}

impl<T: Clone> ReceiverFlavor<T> for CyclicInfiniteReceiver<T> {
    fn attach_receiver(&self, receiver: &dyn Context) {
        self.view_struct.attach_receiver(receiver);
    }

    fn peek(&mut self) -> Recv<T> {
        let recv_time = self.view_struct.receiver_tlb();
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

        let sig_time = self.view_struct.wait_until_sender(recv_time);
        assert!(sig_time >= recv_time);
        self.try_update_head(sig_time);
        self.head.clone()
    }
    fn peek_next(&mut self, manager: &mut TimeManager) -> Recv<T> {
        match &self.head {
            Recv::Something(ce) => {
                manager.advance(ce.time);
                return self.head.clone();
            }
            Recv::Nothing(_) | Recv::Unknown => {}
            Recv::Closed => return Recv::Closed,
        }

        self.head = match self.under_recv() {
            Ok(stuff) => {
                manager.advance(stuff.time);
                Recv::Something(stuff)
            }
            Err(_) => Recv::Closed,
        };

        self.head.clone()
    }

    fn dequeue(&mut self, manager: &mut TimeManager) -> Recv<T> {
        if let Recv::Something(x) = &self.head {
            let time = x.time;
            let mut result = Recv::Unknown;
            std::mem::swap(&mut self.head, &mut result);
            self.view_struct.register_recv();
            manager.advance(time);
            return result;
        }

        if let Recv::Closed = self.head {
            return Recv::Closed;
        }

        // At this point, we can just block!
        match self.under_recv() {
            Ok(ce) => {
                self.view_struct.register_recv();
                manager.advance(ce.time);
                Recv::Something(ce)
            }
            Err(_) => {
                self.head = Recv::Closed;
                Recv::Closed
            }
        }
    }

    fn cleanup(&mut self) {
        self.underlying = ReceiverState::Closed;
    }
}

impl<T: Clone> CyclicInfiniteReceiver<T> {
    pub(crate) fn new(
        underlying: ReceiverState<ChannelElement<T>>,
        view_struct: Arc<ChannelSpec>,
    ) -> Self {
        Self {
            underlying,
            view_struct,
            head: Recv::Unknown,
        }
    }

    fn try_update_head(&mut self, nothing_time: Time) -> bool {
        let mut retflag = false;
        self.head = match self.try_recv() {
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
        retflag
    }

    fn try_recv(&mut self) -> Result<ChannelElement<T>, TryRecvError> {
        match &self.underlying {
            ReceiverState::Open(chan) => chan.try_recv(),
            ReceiverState::Closed => Err(TryRecvError::Disconnected),
        }
    }

    fn under_recv(&mut self) -> Result<ChannelElement<T>, RecvError> {
        match &self.underlying {
            ReceiverState::Open(chan) => chan.recv(),
            ReceiverState::Closed => Err(RecvError),
        }
    }
}
