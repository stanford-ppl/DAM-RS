use crate::{
    channel::{ChannelElement, EnqueueError},
    view::TimeManager,
};

use super::{BoundedProvider, DataProvider, SenderCommon, SenderData, SenderFlavor};

pub(crate) struct UnboundedSender<T> {
    pub(crate) data: SenderData<T>,
}

impl<T> DataProvider<T> for UnboundedSender<T> {
    fn data(&mut self) -> &mut SenderData<T> {
        &mut self.data
    }
}

impl<T> BoundedProvider for UnboundedSender<T> {
    fn register_send(&mut self) {}

    fn wait_until_available(&mut self, _manager: &TimeManager) -> Result<(), EnqueueError> {
        Ok(())
    }
}

impl<T> SenderCommon<T> for UnboundedSender<T> {}

impl<T> SenderFlavor<T> for UnboundedSender<T> {
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
