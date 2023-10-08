use crossbeam::channel::TryRecvError;

use enum_dispatch::enum_dispatch;

use crate::{datastructures::Time, view::TimeManager};

use self::{acyclic::AcyclicReceiver, cyclic::CyclicReceiver};

use super::{channel_spec::InlineSpec, ChannelElement, DequeueError, PeekResult};

mod acyclic;
mod cyclic;
pub mod terminated;
pub mod uninitialized;

#[enum_dispatch(ReceiverImpl<T>)]
pub(super) trait ReceiverFlavor<T> {
    fn peek(&mut self) -> PeekResult<T>;
    fn peek_next(&mut self, manager: &TimeManager) -> Result<ChannelElement<T>, DequeueError>;
    fn dequeue(&mut self, manager: &TimeManager) -> Result<ChannelElement<T>, DequeueError>;
}

#[enum_dispatch]
pub(super) enum ReceiverImpl<T: Clone> {
    // This just tracks metadata
    Uninitialized(uninitialized::UninitializedReceiver),

    // For marking termination and performing GC
    Terminated(terminated::TerminatedReceiver),

    Cyclic(BoundedCyclicReceiver<T>),
    Acyclic(BoundedAcyclicReceiver<T>),
    AcyclicInfinite(InfiniteAcyclicReceiver<T>),
    CyclicInfinite(InfiniteCyclicReceiver<T>),
}

impl<T: Clone> Default for ReceiverImpl<T> {
    fn default() -> Self {
        terminated::TerminatedReceiver::default().into()
    }
}

macro_rules! RegisterReceiver {
    ($name: ident, $receiver_mode: ident) => {
        impl<T> DataProvider<T> for $name<T> {
            fn data(&mut self) -> &mut ReceiverData<T> {
                &mut self.data
            }
        }
        impl<T: Clone> ReceiverCommon<T> for $name<T> {}
        impl<T: Clone> $receiver_mode<T> for $name<T> {}

        impl<T: Clone> ReceiverFlavor<T> for $name<T> {
            fn peek(&mut self) -> PeekResult<T> {
                ReceiverCommon::peek(self)
            }

            fn peek_next(
                &mut self,
                manager: &TimeManager,
            ) -> Result<ChannelElement<T>, DequeueError> {
                $receiver_mode::peek_next(self, manager)
            }

            fn dequeue(
                &mut self,
                manager: &TimeManager,
            ) -> Result<ChannelElement<T>, DequeueError> {
                $receiver_mode::dequeue(self, manager)
            }
        }
    };
}
// Receiver definitions are the cross product of (acyclic, cyclic) x (bounded, unbounded)
// Additionally, we have uninitialized and completed as options as well.

// Holds the basic data for a receiver
pub(super) struct ReceiverData<T> {
    pub(super) spec: InlineSpec,
    pub(super) underlying: crossbeam::channel::Receiver<ChannelElement<T>>,
    pub(super) head: Option<PeekResult<T>>,
}

trait DataProvider<T> {
    fn data(&mut self) -> &mut ReceiverData<T>;
}

trait ReceiverCommon<T: Clone>: Responsive + DataProvider<T> {
    fn peek(&mut self) -> PeekResult<T> {
        let recv_time = self.data().spec.receiver_tlb();
        match &self.data().head {
            Some(PeekResult::Closed) => return PeekResult::Closed,
            Some(PeekResult::Nothing(time)) if *time >= recv_time => {
                // This is a valid nothing
                return PeekResult::Nothing(*time);
            }
            None | Some(PeekResult::Nothing(_)) => {}
            Some(data @ PeekResult::Something(_)) => return data.clone(),
        }
        self.try_update_head(Time::new(0));
        if let Some(result) = &self.data().head {
            return result.clone();
        }

        let sig_time = self.data().spec.wait_until_sender(recv_time);
        assert!(sig_time >= recv_time);
        self.try_update_head(sig_time);
        self.data().head.clone().unwrap()
    }

    fn try_update_head(&mut self, nothing_time: Time) {
        self.data().head = match self.data().underlying.try_recv() {
            Ok(data) => Some(PeekResult::Something(data)),
            Err(TryRecvError::Disconnected) => Some(PeekResult::Closed),
            Err(TryRecvError::Empty) if nothing_time.is_infinite() => Some(PeekResult::Closed),
            Err(TryRecvError::Empty) => Some(PeekResult::Nothing(nothing_time)),
        };
    }
}

trait Responsive {
    fn register_recv(&self, time: Time);
}

trait Unresponsive {}

impl<U> Responsive for U
where
    U: Unresponsive,
{
    fn register_recv(&self, _time: Time) {
        // We did say that we'd be unresponsive.
    }
}

pub(super) struct InfiniteCyclicReceiver<T> {
    pub(super) data: ReceiverData<T>,
}

impl<T> Unresponsive for InfiniteCyclicReceiver<T> {}
RegisterReceiver!(InfiniteCyclicReceiver, CyclicReceiver);

pub(super) struct InfiniteAcyclicReceiver<T> {
    pub(super) data: ReceiverData<T>,
}
impl<T> Unresponsive for InfiniteAcyclicReceiver<T> {}
RegisterReceiver!(InfiniteAcyclicReceiver, AcyclicReceiver);

type ResponseChannel = crossbeam::channel::Sender<Time>;

pub(super) struct BoundedCyclicReceiver<T> {
    pub(super) data: ReceiverData<T>,
    pub(super) resp: ResponseChannel,
}

impl<T> Responsive for BoundedCyclicReceiver<T> {
    fn register_recv(&self, time: Time) {
        let _ = self.resp.send(time + self.data.spec.response_latency);
    }
}
RegisterReceiver!(BoundedCyclicReceiver, CyclicReceiver);

pub(super) struct BoundedAcyclicReceiver<T> {
    pub(super) data: ReceiverData<T>,
    pub(super) resp: ResponseChannel,
}

impl<T> Responsive for BoundedAcyclicReceiver<T> {
    fn register_recv(&self, time: Time) {
        let _ = self.resp.send(time + self.data.spec.response_latency);
    }
}
RegisterReceiver!(BoundedAcyclicReceiver, AcyclicReceiver);
