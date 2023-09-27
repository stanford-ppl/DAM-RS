use std::{marker::PhantomData, sync::Arc};

use dam_core::TimeManager;

use crate::{
    channel::{channel_spec::ChannelSpec, ChannelElement, EnqueueError},
    context::Context,
};

use super::SenderFlavor;

pub struct UninitializedSender<T> {
    _marker: PhantomData<T>,
    spec: Arc<ChannelSpec>,
}
impl<T> SenderFlavor<T> for UninitializedSender<T> {
    fn enqueue(
        &mut self,
        _manager: &mut TimeManager,
        _data: ChannelElement<T>,
    ) -> Result<(), EnqueueError> {
        panic!();
    }

    fn wait_until_available(&mut self, _manager: &mut TimeManager) -> Result<(), EnqueueError> {
        panic!();
    }
}

impl<T> UninitializedSender<T> {
    pub fn new(spec: Arc<ChannelSpec>) -> Self {
        Self {
            _marker: PhantomData,
            spec,
        }
    }

    pub fn attach_sender(&self, sender: &dyn Context) {
        self.spec.attach_sender(sender)
    }
}
