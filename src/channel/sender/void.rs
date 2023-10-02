use std::marker::PhantomData;

use dam_core::prelude::*;

use crate::channel::{ChannelElement, EnqueueError};

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
        _manager: &mut TimeManager,
        _data: ChannelElement<T>,
    ) -> Result<(), EnqueueError> {
        Ok(())
    }

    fn wait_until_available(&mut self, _manager: &mut TimeManager) -> Result<(), EnqueueError> {
        // No-op
        Ok(())
    }
}
