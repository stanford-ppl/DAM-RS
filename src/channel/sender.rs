use std::{marker::PhantomData, sync::Arc};

use crossbeam::channel;
use dam_core::{time::Time, TimeManager};
use dam_macros::log_producer;
use enum_dispatch::enum_dispatch;

use crate::context::Context;

use super::{view_struct::ViewStruct, ChannelElement, EnqueueError};

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
pub(super) enum SenderImpl<T: Clone> {
    VoidSender(VoidSender<T>),
    CyclicSender(CyclicSender<T>),
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

pub(super) enum SenderState<T> {
    Open(channel::Sender<T>),
    Closed,
}

#[log_producer]
pub(super) struct CyclicSender<T> {
    pub(super) underlying: SenderState<ChannelElement<T>>,
    pub(super) resp: channel::Receiver<Time>,
    pub(super) send_receive_delta: usize,
    pub(super) capacity: usize,

    pub(super) view_struct: Arc<ViewStruct>,
    pub(super) next_available: SendOptions,
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
                    unreachable!("We should always know when to try again!")
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
