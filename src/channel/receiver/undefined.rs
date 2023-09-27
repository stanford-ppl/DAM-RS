use std::sync::Arc;

use dam_core::TimeManager;

use crate::{
    channel::{channel_spec::ChannelSpec, DequeueResult, PeekResult},
    context::Context,
};

use super::ReceiverFlavor;

pub struct UndefinedReceiver {
    spec: Arc<ChannelSpec>,
}

impl UndefinedReceiver {
    pub fn new(spec: Arc<ChannelSpec>) -> Self {
        Self { spec }
    }
}

impl<T> ReceiverFlavor<T> for UndefinedReceiver {
    fn peek(&mut self) -> PeekResult<T> {
        panic!();
    }

    fn peek_next(&mut self, _manager: &mut TimeManager) -> DequeueResult<T> {
        panic!();
    }

    fn dequeue(&mut self, _manager: &mut TimeManager) -> DequeueResult<T> {
        panic!();
    }
}

impl UndefinedReceiver {
    pub fn attach_receiver(&self, receiver: &dyn Context) {
        self.spec.attach_receiver(receiver);
    }
}
