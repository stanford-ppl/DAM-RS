use std::{marker::PhantomData, sync::Arc};

use crossbeam::channel;
use dam_core::{time::Time, TimeManager};
use dam_macros::log_producer;
use enum_dispatch::enum_dispatch;

use crate::context::Context;

use super::{view_struct::ChannelSpec, ChannelElement, EnqueueError};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SendOptions {
    Unknown,
    AvailableAt(Time),
    CheckBackAt(Time),
    Never,
}

#[enum_dispatch(SenderImpl<T>)]
pub trait SenderFlavor<T> {
    fn attach_sender(&self, sender: &dyn Context);

    fn try_send(&mut self, data: ChannelElement<T>) -> Result<(), SendOptions>;

    fn enqueue(
        &mut self,
        manager: &mut TimeManager,
        data: ChannelElement<T>,
    ) -> Result<(), EnqueueError>;

    fn cleanup(&mut self);
}

#[enum_dispatch]
pub(crate) enum SenderImpl<T: Clone> {
    VoidSender(VoidSender<T>),
    CyclicSender(CyclicSender<T>),
    AcyclicSender(AcyclicSender<T>),
    InfiniteSender(InfiniteSender<T>),
    UndefinedSender(UndefinedSender<T>),
}

#[derive(Debug)]
pub struct VoidSender<T> {
    _marker: PhantomData<T>,
}

impl<T> Default for VoidSender<T> {
    fn default() -> Self {
        Self {
            _marker: Default::default(),
        }
    }
}

impl<T> SenderFlavor<T> for VoidSender<T> {
    fn attach_sender(&self, _sender: &dyn Context) {
        // No-op. We really don't care.
    }

    fn try_send(&mut self, _data: ChannelElement<T>) -> Result<(), SendOptions> {
        Ok(())
    }

    fn enqueue(
        &mut self,
        _manager: &mut TimeManager,
        _data: ChannelElement<T>,
    ) -> Result<(), EnqueueError> {
        Ok(())
    }

    fn cleanup(&mut self) {} // Nothing to clean up either.
}

pub struct UndefinedSender<T> {
    _marker: PhantomData<T>,
    spec: Arc<ChannelSpec>,
}
impl<T> SenderFlavor<T> for UndefinedSender<T> {
    fn attach_sender(&self, sender: &dyn Context) {
        self.spec.attach_sender(sender)
    }

    fn try_send(&mut self, _data: ChannelElement<T>) -> Result<(), SendOptions> {
        panic!();
    }

    fn enqueue(
        &mut self,
        _manager: &mut TimeManager,
        _data: ChannelElement<T>,
    ) -> Result<(), EnqueueError> {
        panic!();
    }

    fn cleanup(&mut self) {
        panic!();
    }
}

impl<T> UndefinedSender<T> {
    pub fn new(spec: Arc<ChannelSpec>) -> Self {
        Self {
            _marker: PhantomData,
            spec,
        }
    }
}

pub(crate) enum SenderState<T> {
    Open(channel::Sender<T>),
    Closed,
}

#[log_producer]
pub(crate) struct CyclicSender<T> {
    underlying: SenderState<ChannelElement<T>>,
    resp: channel::Receiver<Time>,
    send_receive_delta: usize,
    capacity: usize,

    view_struct: Arc<ChannelSpec>,
    next_available: SendOptions,
}
impl<T: Clone> SenderFlavor<T> for CyclicSender<T> {
    fn attach_sender(&self, sender: &dyn Context) {
        self.view_struct.attach_sender(sender);
    }

    fn try_send(&mut self, elem: ChannelElement<T>) -> Result<(), SendOptions> {
        if self.is_full() {
            return Err(self.next_available);
        }

        assert!(self.send_receive_delta < self.capacity);
        assert!(elem.time >= self.view_struct.sender_tlb());
        let prev_srd = self.view_struct.register_send();
        assert!(prev_srd < self.capacity);
        self.under_send(elem).unwrap();
        self.send_receive_delta += 1;

        Ok(())
    }

    fn enqueue(
        &mut self,
        manager: &mut TimeManager,
        data: ChannelElement<T>,
    ) -> Result<(), EnqueueError> {
        let mut data_copy = data.clone();
        loop {
            data_copy.update_time(manager.tick() + 1);
            let v = self.try_send(data_copy.clone());
            match v {
                Ok(()) => return Ok(()),
                Err(SendOptions::Never) => {
                    return Err(EnqueueError {});
                }
                Err(SendOptions::CheckBackAt(time)) | Err(SendOptions::AvailableAt(time)) => {
                    // Have to make sure that we're making progress
                    assert!(time > manager.tick());
                    manager.advance(time);
                }
                Err(SendOptions::Unknown) => {
                    panic!("We should always know when to try again!")
                }
            }
        }
    }

    fn cleanup(&mut self) {
        self.underlying = SenderState::Closed;
    }
}

impl<T> CyclicSender<T> {
    fn under_send(
        &mut self,
        elem: ChannelElement<T>,
    ) -> Result<(), channel::SendError<ChannelElement<T>>> {
        match &self.underlying {
            SenderState::Open(sender) => sender.send(elem),
            SenderState::Closed => Err(channel::SendError(elem)),
        }
    }

    fn is_full(&mut self) -> bool {
        if self.send_receive_delta < self.capacity {
            return false;
        }
        self.update_len();

        self.send_receive_delta == self.capacity
    }

    fn update_srd(&mut self) -> bool {
        let send_time = self.view_struct.sender_tlb();
        // We don't know when it'll be available.
        self.next_available = SendOptions::Unknown;

        let real_srd = self.view_struct.current_srd();
        assert!(real_srd <= self.send_receive_delta);
        let srd_diff = self.send_receive_delta - real_srd;

        let mut retval = false;

        for _ in 0..srd_diff {
            match self.resp.recv() {
                Ok(time) if time <= send_time => {
                    assert!(self.send_receive_delta > 0);
                    self.send_receive_delta -= 1;
                    retval = true;
                }
                Ok(time) => {
                    // Got a time in the future
                    assert!(self.next_available == SendOptions::Unknown);
                    self.next_available = SendOptions::AvailableAt(time);
                    return true;
                }
                Err(channel::RecvError) => {
                    self.next_available = SendOptions::Never;
                    return true;
                }
            }
        }
        return retval;
    }

    fn update_len(&mut self) {
        let send_time = self.view_struct.sender_tlb();

        match self.next_available {
            SendOptions::Never => return,
            SendOptions::AvailableAt(time) if time <= send_time => {
                // Next available time has already passed, so we pop an element off.
                // Additionally, to avoid work, we don't update next_available immediately.
                self.next_available = SendOptions::Unknown;
                assert_ne!(self.send_receive_delta, 0);
                self.send_receive_delta -= 1;
                return;
            }

            // If we were supposed to check back in sometime in the past, or we don't know, then we continue.
            SendOptions::CheckBackAt(time) if time <= send_time => {}
            SendOptions::Unknown => {}

            // In these cases, we were already told to check back in the future.
            SendOptions::AvailableAt(_) | SendOptions::CheckBackAt(_) => {
                return;
            }
        }

        if self.update_srd() {
            return;
        }

        let new_time = self.view_struct.wait_until_receiver(send_time);

        // Forces the resp channel to synchronize w.r.t. the signal.

        self.update_srd();
        if self.next_available == SendOptions::Unknown {
            self.next_available = SendOptions::CheckBackAt(new_time + 1)
        }
    }
}

impl<T> CyclicSender<T> {
    pub(crate) fn new(
        sender: channel::Sender<ChannelElement<T>>,
        resp: channel::Receiver<Time>,
        capacity: usize,
        view_struct: Arc<ChannelSpec>,
    ) -> Self {
        Self {
            underlying: SenderState::Open(sender),
            resp,
            send_receive_delta: 0,
            capacity,
            view_struct,
            next_available: SendOptions::Unknown,
        }
    }
}

pub(crate) struct AcyclicSender<T> {
    underlying: SenderState<ChannelElement<T>>,
    resp: channel::Receiver<Time>,
    send_receive_delta: usize,
    capacity: usize,

    view_struct: Arc<ChannelSpec>,
    next_available: SendOptions,
}

impl<T: Clone> SenderFlavor<T> for AcyclicSender<T> {
    fn attach_sender(&self, sender: &dyn Context) {
        self.view_struct.attach_sender(sender)
    }

    fn try_send(&mut self, data: ChannelElement<T>) -> Result<(), SendOptions> {
        if self.send_receive_delta == self.capacity {
            let sender_time = self.view_struct.sender_tlb();
            match self.next_available {
                SendOptions::AvailableAt(time) if time > sender_time => {
                    return Err(self.next_available);
                }
                SendOptions::Never => return Err(SendOptions::Never),

                // Unknown is the base state.
                SendOptions::Unknown => {
                    let new_time = self.resp.recv().unwrap();
                    if new_time <= sender_time {
                        self.send_receive_delta -= 1;
                    } else {
                        self.next_available = SendOptions::AvailableAt(new_time);
                        return Err(self.next_available);
                    }
                }

                // We're ready, so we pop the availability and continue with the write.
                SendOptions::AvailableAt(_) => {
                    self.next_available = SendOptions::Unknown;
                    self.send_receive_delta -= 1;
                }

                SendOptions::CheckBackAt(_) => {
                    panic!("We should never have to check back in an acyclic sender");
                }
            }
        }
        assert!(self.send_receive_delta < self.capacity);
        self.view_struct.register_send();
        // Not full, proceed.
        match &self.underlying {
            SenderState::Open(sender) => match sender.send(data) {
                Ok(_) => {
                    self.send_receive_delta += 1;
                    Ok(())
                }
                Err(_) => {
                    self.underlying = SenderState::Closed;
                    self.next_available = SendOptions::Never;
                    Err(SendOptions::Never)
                } // Channel is closed
            },
            SenderState::Closed => {
                self.underlying = SenderState::Closed;
                self.next_available = SendOptions::Never;
                Err(SendOptions::Never)
            }
        }
    }

    fn enqueue(
        &mut self,
        manager: &mut TimeManager,
        data: ChannelElement<T>,
    ) -> Result<(), EnqueueError> {
        let mut data_clone = data.clone();
        data_clone.update_time(manager.tick() + 1);
        match self.try_send(data_clone.clone()) {
            Ok(_) => Ok(()),
            Err(SendOptions::AvailableAt(time)) => {
                manager.advance(time);
                data_clone.update_time(time + 1);
                self.try_send(data_clone)
                    .expect("Should have succeeded on the second attempt!");
                Ok(())
            }
            Err(SendOptions::Never) => Err(EnqueueError {}),
            Err(_) => panic!("Not possible to get an Unknown or CheckBackAt"),
        }
    }

    fn cleanup(&mut self) {
        self.underlying = SenderState::Closed;
    }
}

impl<T> AcyclicSender<T> {
    pub(crate) fn new(
        sender: channel::Sender<ChannelElement<T>>,
        resp: channel::Receiver<Time>,
        capacity: usize,
        view_struct: Arc<ChannelSpec>,
    ) -> Self {
        Self {
            underlying: SenderState::Open(sender),
            resp,
            send_receive_delta: 0,
            capacity,
            view_struct,
            next_available: SendOptions::Unknown,
        }
    }
}

pub(crate) struct InfiniteSender<T> {
    underlying: SenderState<ChannelElement<T>>,
    view_struct: Arc<ChannelSpec>,
}

impl<T> InfiniteSender<T> {
    pub(crate) fn new(
        underlying: SenderState<ChannelElement<T>>,
        view_struct: Arc<ChannelSpec>,
    ) -> Self {
        Self {
            underlying,
            view_struct,
        }
    }
}

impl<T: Clone> SenderFlavor<T> for InfiniteSender<T> {
    fn attach_sender(&self, sender: &dyn Context) {
        self.view_struct.attach_sender(sender);
    }

    fn try_send(&mut self, elem: ChannelElement<T>) -> Result<(), SendOptions> {
        assert!(elem.time >= self.view_struct.sender_tlb());
        let _ = self.view_struct.register_send();
        match &self.underlying {
            SenderState::Open(chan) => match chan.send(elem) {
                Ok(_) => Ok(()),
                Err(_) => Err(SendOptions::Never),
            },
            SenderState::Closed => Err(SendOptions::Never),
        }
    }

    fn enqueue(
        &mut self,
        manager: &mut TimeManager,
        data: ChannelElement<T>,
    ) -> Result<(), EnqueueError> {
        let mut data_copy = data.clone();
        data_copy.update_time(manager.tick() + 1);
        self.try_send(data_copy).map_err(|_| EnqueueError {})
    }

    fn cleanup(&mut self) {
        self.underlying = SenderState::Closed;
    }
}
