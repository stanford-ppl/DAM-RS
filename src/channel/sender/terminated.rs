use std::marker::PhantomData;

use crate::{
    channel::{ChannelElement, EnqueueError},
    view::TimeManager,
};

use super::SenderFlavor;

pub struct TerminatedSender<T> {
    _marker: PhantomData<T>,
}
impl<T> SenderFlavor<T> for TerminatedSender<T> {
    fn enqueue(
        &mut self,
        _manager: &TimeManager,
        _data: ChannelElement<T>,
    ) -> Result<(), EnqueueError> {
        panic!("Attempting to enqueue to a terminated sender.");
    }

    fn wait_until_available(&mut self, _manager: &TimeManager) -> Result<(), EnqueueError> {
        panic!("Attempting to wait for a terminated sender.");
    }
}

impl<T> Default for TerminatedSender<T> {
    fn default() -> Self {
        Self {
            _marker: Default::default(),
        }
    }
}
