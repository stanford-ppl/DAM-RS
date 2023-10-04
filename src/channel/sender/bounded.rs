use crate::channel::{ChannelElement, EnqueueError};
use crossbeam::channel;
use dam_core::prelude::*;

use super::{BoundedProvider, DataProvider, SenderCommon, SenderData, SenderFlavor};

pub(crate) struct BoundedData {
    pub(crate) resp: channel::Receiver<Time>,
    pub(crate) send_receive_delta: usize,
}

pub(crate) struct BoundedAcyclicSender<T> {
    pub(crate) data: SenderData<T>,
    pub(crate) bound: BoundedData,
}

impl<T> DataProvider<T> for BoundedAcyclicSender<T> {
    fn data(&mut self) -> &mut SenderData<T> {
        &mut self.data
    }
}

impl<T> BoundedProvider for BoundedAcyclicSender<T> {
    fn register_send(&mut self) {
        self.bound.send_receive_delta += 1;
    }

    fn wait_until_available(&mut self, manager: &TimeManager) -> Result<(), EnqueueError> {
        if self.bound.send_receive_delta < self.data.spec.capacity.unwrap() {
            return Ok(());
        }
        match self.bound.resp.recv() {
            Ok(time) => {
                manager.advance(time);
                Ok(())
            }
            Err(_) => Err(EnqueueError::Closed),
        }
    }
}
impl<T> SenderCommon<T> for BoundedAcyclicSender<T> {}

impl<T> SenderFlavor<T> for BoundedAcyclicSender<T> {
    fn wait_until_available(&mut self, manager: &TimeManager) -> Result<(), EnqueueError> {
        BoundedProvider::wait_until_available(self, manager)
    }

    fn enqueue(
        &mut self,
        manager: &TimeManager,
        data: ChannelElement<T>,
    ) -> Result<(), EnqueueError> {
        SenderCommon::enqueue(self, manager, data)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SendOptions {
    AvailableAt(Time),
    CheckBackAt(Time),
    Never,
}
pub(crate) struct BoundedCyclicSender<T> {
    pub(crate) data: SenderData<T>,
    pub(crate) bound: BoundedData,
    pub(crate) next_available: Option<SendOptions>,
}

impl<T> BoundedCyclicSender<T> {
    fn update_srd(&mut self) -> bool {
        let send_time = self.data.spec.sender_tlb();
        // We don't know when it'll be available.
        self.next_available = None;

        let mut retval = false;

        loop {
            match self.bound.resp.try_recv() {
                Ok(time) if time <= send_time => {
                    assert!(self.bound.send_receive_delta > 0);
                    self.bound.send_receive_delta -= 1;
                    retval = true;
                }
                Ok(time) => {
                    // Got a time in the future
                    assert!(self.next_available.is_none());
                    self.next_available = Some(SendOptions::AvailableAt(time));
                    return true;
                }
                Err(channel::TryRecvError::Empty) => {
                    return retval;
                }
                Err(channel::TryRecvError::Disconnected) => {
                    assert!(self.next_available.is_none());
                    self.next_available = Some(SendOptions::Never);
                    return true;
                }
            }
        }
    }
}

impl<T> BoundedProvider for BoundedCyclicSender<T> {
    fn register_send(&mut self) {
        self.bound.send_receive_delta += 1;
    }

    fn wait_until_available(&mut self, manager: &TimeManager) -> Result<(), EnqueueError> {
        loop {
            if self.bound.send_receive_delta < self.data.spec.capacity.unwrap() {
                return Ok(());
            }
            match self.next_available {
                Some(SendOptions::AvailableAt(time)) => {
                    manager.advance(time);
                    self.bound.send_receive_delta -= 1;
                    self.next_available = None;
                    return Ok(());
                }
                Some(SendOptions::Never) => {
                    return Err(EnqueueError::Closed);
                }
                Some(SendOptions::CheckBackAt(time)) => {
                    manager.advance(time);
                    self.next_available = None;
                }
                None => {}
            }

            if self.update_srd() {
                continue;
            }

            let new_time = self.data.spec.wait_until_receiver(manager.tick());

            // Forces the resp channel to synchronize w.r.t. the signal.

            if !self.update_srd() {
                self.next_available = Some(SendOptions::CheckBackAt(
                    new_time + self.data.spec.response_latency,
                ));
            }
        }
    }
}
impl<T> DataProvider<T> for BoundedCyclicSender<T> {
    fn data(&mut self) -> &mut SenderData<T> {
        &mut self.data
    }
}
impl<T> SenderCommon<T> for BoundedCyclicSender<T> {}

impl<T> SenderFlavor<T> for BoundedCyclicSender<T> {
    fn wait_until_available(&mut self, manager: &TimeManager) -> Result<(), EnqueueError> {
        BoundedProvider::wait_until_available(self, manager)
    }

    fn enqueue(
        &mut self,
        manager: &TimeManager,
        data: ChannelElement<T>,
    ) -> Result<(), EnqueueError> {
        SenderCommon::enqueue(self, manager, data)
    }
}
