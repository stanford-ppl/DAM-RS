use dam_core::TimeManager;

use enum_dispatch::enum_dispatch;

use self::{
    bounded::{BoundedAcyclicSender, BoundedCyclicSender},
    unbounded::UnboundedSender,
};

use super::{channel_spec::InlineSpec, ChannelElement, EnqueueError};

pub(super) mod bounded;
pub(super) mod terminated;
pub(super) mod unbounded;
pub(super) mod uninitialized;
pub(super) mod void;

#[enum_dispatch(SenderImpl<T>)]
pub trait SenderFlavor<T> {
    fn wait_until_available(&mut self, manager: &mut TimeManager) -> Result<(), EnqueueError>;

    fn enqueue(
        &mut self,
        manager: &mut TimeManager,
        data: ChannelElement<T>,
    ) -> Result<(), EnqueueError>;
}

#[enum_dispatch]
pub(super) enum SenderImpl<T: Clone> {
    Uninitialized(uninitialized::UninitializedSender<T>),

    Terminated(terminated::TerminatedSender<T>),

    Void(void::VoidSender<T>),
    Cyclic(BoundedCyclicSender<T>),
    Acyclic(BoundedAcyclicSender<T>),
    Infinite(UnboundedSender<T>),
}

pub(crate) struct SenderData<T> {
    pub(crate) spec: InlineSpec,
    pub(crate) underlying: crossbeam::channel::Sender<ChannelElement<T>>,
}

trait DataProvider<T> {
    fn data(&mut self) -> &mut SenderData<T>;
}

trait BoundedProvider {
    fn register_send(&mut self);
    fn wait_until_available(&mut self, manager: &mut TimeManager) -> Result<(), EnqueueError>;
}

trait SenderCommon<T>: DataProvider<T> + BoundedProvider {
    fn enqueue(
        &mut self,
        manager: &mut TimeManager,
        mut data: ChannelElement<T>,
    ) -> Result<(), EnqueueError> {
        if let err @ Err(_) = self.wait_until_available(manager) {
            return err;
        }
        data.update_time(manager.tick() + self.data().spec.send_latency);
        self.data().underlying.send(data).unwrap();
        self.register_send();
        Ok(())
    }
}

// pub(crate) enum SenderState<T> {
//     Open(channel::Sender<T>),
//     Closed,
// }

// #[log_producer]
// pub(crate) struct CyclicSender<T> {
//     underlying: SenderState<ChannelElement<T>>,
//     resp: channel::Receiver<Time>,
//     send_receive_delta: usize,
//     capacity: usize,
//     latency: u64,
//     resp_latency: u64,

//     view_struct: InlineSpec,
//     next_available: SendOptions,
// }

// impl<T: Clone> SenderFlavor<T> for CyclicSender<T> {
//     fn try_send(&mut self, elem: ChannelElement<T>) -> Result<(), SendOptions> {
//         if self.is_full() {
//             return Err(self.next_available);
//         }

//         assert!(self.send_receive_delta < self.capacity);
//         assert!(elem.time >= self.view_struct.sender_tlb());
//         self.under_send(elem).unwrap();
//         self.send_receive_delta += 1;

//         Ok(())
//     }

//     fn enqueue(
//         &mut self,
//         manager: &mut TimeManager,
//         data: ChannelElement<T>,
//     ) -> Result<(), EnqueueError> {
//         let mut data_copy = data;
//         loop {
//             data_copy.update_time(manager.tick() + 1);
//             let v = self.try_send(data_copy.clone());
//             match v {
//                 Ok(()) => return Ok(()),
//                 Err(SendOptions::Never) => {
//                     return Err(EnqueueError {});
//                 }
//                 Err(SendOptions::CheckBackAt(time)) | Err(SendOptions::AvailableAt(time)) => {
//                     // Have to make sure that we're making progress
//                     assert!(time > manager.tick());
//                     manager.advance(time);
//                 }
//                 Err(SendOptions::Unknown) => {
//                     panic!("We should always know when to try again!")
//                 }
//             }
//         }
//     }
//     fn wait_until_available(&mut self, manager: &mut TimeManager) -> Result<(), EnqueueError> {
//         loop {
//             if !self.is_full() {
//                 return Ok(());
//             }
//             match self.next_available {
//                 SendOptions::Never => {
//                     return Err(EnqueueError {});
//                 }
//                 SendOptions::CheckBackAt(time) | SendOptions::AvailableAt(time) => {
//                     // Have to make sure that we're making progress
//                     assert!(time > manager.tick());
//                     manager.advance(time);
//                 }
//                 SendOptions::Unknown => {
//                     panic!("We should always know when to try again!")
//                 }
//             }
//         }
//     }
// }

// impl<T> CyclicSender<T> {
//     fn under_send(
//         &mut self,
//         elem: ChannelElement<T>,
//     ) -> Result<(), channel::SendError<ChannelElement<T>>> {
//         match &self.underlying {
//             SenderState::Open(sender) => sender.send(elem),
//             SenderState::Closed => Err(channel::SendError(elem)),
//         }
//     }

//     fn is_full(&mut self) -> bool {
//         if self.send_receive_delta < self.capacity {
//             return false;
//         }
//         self.update_len();

//         self.send_receive_delta == self.capacity
//     }

//     fn update_srd(&mut self) -> bool {
//         let send_time = self.view_struct.sender_tlb();
//         // We don't know when it'll be available.
//         self.next_available = SendOptions::Unknown;

//         let mut retval = false;

//         loop {
//             match self.resp.try_recv() {
//                 Ok(time) if time <= send_time => {
//                     assert!(self.send_receive_delta > 0);
//                     self.send_receive_delta -= 1;
//                     retval = true;
//                 }
//                 Ok(time) => {
//                     // Got a time in the future
//                     assert!(self.next_available == SendOptions::Unknown);
//                     self.next_available = SendOptions::AvailableAt(time);
//                     return true;
//                 }
//                 Err(channel::TryRecvError::Empty) => {
//                     return retval;
//                 }
//                 Err(channel::TryRecvError::Disconnected) => {
//                     assert!(self.next_available == SendOptions::Unknown);
//                     self.next_available = SendOptions::Never;
//                     return true;
//                 }
//             }
//         }
//     }

//     fn update_len(&mut self) {
//         let send_time = self.view_struct.sender_tlb();

//         match self.next_available {
//             SendOptions::Never => return,
//             SendOptions::AvailableAt(time) if time <= send_time => {
//                 // Next available time has already passed, so we pop an element off.
//                 // Additionally, to avoid work, we don't update next_available immediately.
//                 self.next_available = SendOptions::Unknown;
//                 assert_ne!(self.send_receive_delta, 0);
//                 self.send_receive_delta -= 1;
//                 return;
//             }

//             // If we were supposed to check back in sometime in the past, or we don't know, then we continue.
//             SendOptions::CheckBackAt(time) if time <= send_time => {}
//             SendOptions::Unknown => {}

//             // In these cases, we were already told to check back in the future.
//             SendOptions::AvailableAt(_) | SendOptions::CheckBackAt(_) => {
//                 return;
//             }
//         }

//         if self.update_srd() {
//             return;
//         }

//         let new_time = self.view_struct.wait_until_receiver(send_time);

//         // Forces the resp channel to synchronize w.r.t. the signal.

//         self.update_srd();
//         if self.next_available == SendOptions::Unknown {
//             self.next_available = SendOptions::CheckBackAt(new_time + self.resp_latency)
//         }
//     }
// }

// impl<T> CyclicSender<T> {
//     pub(crate) fn new(
//         sender: channel::Sender<ChannelElement<T>>,
//         resp: channel::Receiver<Time>,
//         view_struct: InlineSpec,
//     ) -> Self {
//         Self {
//             underlying: SenderState::Open(sender),
//             resp,
//             send_receive_delta: 0,
//             capacity: view_struct.capacity.unwrap(),
//             latency: view_struct.send_latency,
//             resp_latency: view_struct.response_latency,
//             view_struct,
//             next_available: SendOptions::Unknown,
//         }
//     }
// }

// pub(crate) struct AcyclicSender<T> {
//     underlying: SenderState<ChannelElement<T>>,
//     resp: channel::Receiver<Time>,
//     send_receive_delta: usize,

//     view_struct: InlineSpec,
//     next_available: SendOptions,
// }

// impl<T: Clone> SenderFlavor<T> for AcyclicSender<T> {
//     fn try_send(&mut self, data: ChannelElement<T>) -> Result<(), SendOptions> {
//         if self.send_receive_delta == self.view_struct.capacity.unwrap() {
//             let sender_time = self.view_struct.sender_tlb();
//             match self.next_available {
//                 SendOptions::AvailableAt(time) if time > sender_time => {
//                     return Err(self.next_available);
//                 }
//                 SendOptions::Never => return Err(SendOptions::Never),

//                 // Unknown is the base state.
//                 SendOptions::Unknown => {
//                     let new_time = self.resp.recv().unwrap();
//                     if new_time <= sender_time {
//                         self.send_receive_delta -= 1;
//                     } else {
//                         self.next_available = SendOptions::AvailableAt(new_time);
//                         return Err(self.next_available);
//                     }
//                 }

//                 // We're ready, so we pop the availability and continue with the write.
//                 SendOptions::AvailableAt(_) => {
//                     self.next_available = SendOptions::Unknown;
//                     self.send_receive_delta -= 1;
//                 }

//                 SendOptions::CheckBackAt(_) => {
//                     panic!("We should never have to check back in an acyclic sender");
//                 }
//             }
//         }
//         assert!(self.send_receive_delta < self.view_struct.capacity.unwrap());
//         // Not full, proceed.
//         match &self.underlying {
//             SenderState::Open(sender) => match sender.send(data) {
//                 Ok(_) => {
//                     self.send_receive_delta += 1;
//                     Ok(())
//                 }
//                 Err(_) => {
//                     self.underlying = SenderState::Closed;
//                     self.next_available = SendOptions::Never;
//                     Err(SendOptions::Never)
//                 } // Channel is closed
//             },
//             SenderState::Closed => {
//                 self.underlying = SenderState::Closed;
//                 self.next_available = SendOptions::Never;
//                 Err(SendOptions::Never)
//             }
//         }
//     }

//     fn enqueue(
//         &mut self,
//         manager: &mut TimeManager,
//         data: ChannelElement<T>,
//     ) -> Result<(), EnqueueError> {
//         let mut data_clone = data;
//         data_clone.update_time(manager.tick() + 1);
//         match self.try_send(data_clone.clone()) {
//             Ok(_) => Ok(()),
//             Err(SendOptions::AvailableAt(time)) => {
//                 manager.advance(time);
//                 data_clone.update_time(time + 1);
//                 self.try_send(data_clone)
//                     .expect("Should have succeeded on the second attempt!");
//                 Ok(())
//             }
//             Err(SendOptions::Never) => Err(EnqueueError {}),
//             Err(_) => panic!("Not possible to get an Unknown or CheckBackAt"),
//         }
//     }

//     fn wait_until_available(&mut self, manager: &mut TimeManager) -> Result<(), EnqueueError> {
//         if self.send_receive_delta == self.view_struct.capacity.unwrap() {
//             match self.next_available {
//                 SendOptions::Never => return Err(EnqueueError {}),

//                 // Unknown is the base state.
//                 SendOptions::Unknown => {
//                     let new_time = self.resp.recv().unwrap();
//                     self.send_receive_delta -= 1;
//                     manager.advance(new_time);
//                 }

//                 // We're ready, so we pop the availability and continue with the write.
//                 SendOptions::AvailableAt(time) => {
//                     manager.advance(time);
//                     self.next_available = SendOptions::Unknown;
//                     self.send_receive_delta -= 1;
//                 }

//                 SendOptions::CheckBackAt(_) => {
//                     panic!("We should never have to check back in an acyclic sender");
//                 }
//             }
//         }
//         Ok(())
//     }
// }

// impl<T> AcyclicSender<T> {
//     pub(crate) fn new(
//         sender: channel::Sender<ChannelElement<T>>,
//         resp: channel::Receiver<Time>,
//         view_struct: InlineSpec,
//     ) -> Self {
//         Self {
//             underlying: SenderState::Open(sender),
//             resp,
//             send_receive_delta: 0,
//             view_struct,
//             next_available: SendOptions::Unknown,
//         }
//     }
// }

// pub(crate) struct InfiniteSender<T> {
//     underlying: SenderState<ChannelElement<T>>,
//     view_struct: InlineSpec,
// }

// impl<T> InfiniteSender<T> {
//     pub(crate) fn new(underlying: SenderState<ChannelElement<T>>, view_struct: InlineSpec) -> Self {
//         Self {
//             underlying,
//             view_struct,
//         }
//     }
// }

// impl<T: Clone> SenderFlavor<T> for InfiniteSender<T> {
//     fn try_send(&mut self, elem: ChannelElement<T>) -> Result<(), SendOptions> {
//         assert!(elem.time >= self.view_struct.sender_tlb());
//         match &self.underlying {
//             SenderState::Open(chan) => match chan.send(elem) {
//                 Ok(_) => Ok(()),
//                 Err(_) => Err(SendOptions::Never),
//             },
//             SenderState::Closed => Err(SendOptions::Never),
//         }
//     }

//     fn enqueue(
//         &mut self,
//         manager: &mut TimeManager,
//         data: ChannelElement<T>,
//     ) -> Result<(), EnqueueError> {
//         let mut data_copy = data;
//         data_copy.update_time(manager.tick() + 1);
//         self.try_send(data_copy).map_err(|_| EnqueueError {})
//     }

//     fn wait_until_available(&mut self, _manager: &mut TimeManager) -> Result<(), EnqueueError> {
//         Ok(())
//     }
// }
