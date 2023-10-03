use dam_core::prelude::*;

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
    fn wait_until_available(&mut self, manager: &TimeManager) -> Result<(), EnqueueError>;

    fn enqueue(
        &mut self,
        manager: &TimeManager,
        data: ChannelElement<T>,
    ) -> Result<(), EnqueueError>;
}

#[enum_dispatch]
pub(super) enum SenderImpl<T> {
    Uninitialized(uninitialized::UninitializedSender<T>),

    // Terminated senders are also used to help with default initialization
    Terminated(terminated::TerminatedSender<T>),

    Void(void::VoidSender<T>),
    Cyclic(BoundedCyclicSender<T>),
    Acyclic(BoundedAcyclicSender<T>),
    Infinite(UnboundedSender<T>),
}

impl<T: Clone> Default for SenderImpl<T> {
    fn default() -> Self {
        terminated::TerminatedSender::default().into()
    }
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
    fn wait_until_available(&mut self, manager: &TimeManager) -> Result<(), EnqueueError>;
}

trait SenderCommon<T>: DataProvider<T> + BoundedProvider {
    fn enqueue(
        &mut self,
        manager: &TimeManager,
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
