use std::marker::PhantomData;

/// Similar to PhantomData, but does NOT imply that this may own an instance, and is purely used to capture type parameters.
#[derive(Clone, Copy, Debug)]
pub struct SyncSendMarker<T> {
    _marker: PhantomData<T>,
}

unsafe impl<T> Sync for SyncSendMarker<T> {}
unsafe impl<T> Send for SyncSendMarker<T> {}

impl<T> Default for SyncSendMarker<T> {
    fn default() -> Self {
        Self {
            _marker: Default::default(),
        }
    }
}
