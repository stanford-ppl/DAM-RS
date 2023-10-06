use std::sync::Arc;

use dam_core::prelude::*;

use crate::{
    channel::{channel_spec::ChannelSpec, ChannelElement, DequeueError, PeekResult},
    context::Context,
};

use super::ReceiverFlavor;

pub struct UninitializedReceiver {
    spec: Arc<ChannelSpec>,
}

impl UninitializedReceiver {
    pub fn new(spec: Arc<ChannelSpec>) -> Self {
        Self { spec }
    }
}

impl<T> ReceiverFlavor<T> for UninitializedReceiver {
    fn peek(&mut self) -> PeekResult<T> {
        panic!("Calling peek on an uninitialized receiver");
    }

    fn peek_next(&mut self, _manager: &TimeManager) -> Result<ChannelElement<T>, DequeueError> {
        panic!("Calling peek_next on an uninitialized receiver");
    }

    fn dequeue(&mut self, _manager: &TimeManager) -> Result<ChannelElement<T>, DequeueError> {
        panic!("Calling dequeue on an uninitialized receiver");
    }
}

impl UninitializedReceiver {
    pub fn attach_receiver(&self, receiver: &dyn Context) {
        self.spec.attach_receiver(receiver);
    }
}
