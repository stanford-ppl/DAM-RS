use std::marker::PhantomData;

use crate::{
    channel::{ChannelElement, EnqueueError},
    view::TimeManager,
};

use super::SenderFlavor;

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
    fn enqueue(
        &mut self,
        _manager: &TimeManager,
        _data: ChannelElement<T>,
    ) -> Result<(), EnqueueError> {
        Ok(())
    }

    fn wait_until_available(&mut self, _manager: &TimeManager) -> Result<(), EnqueueError> {
        // No-op
        Ok(())
    }
}
