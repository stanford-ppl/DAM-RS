use std::sync::atomic::AtomicUsize;

use serde::{Deserialize, Serialize};

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub struct ChannelID {
    id: usize,
}

impl ChannelID {
    fn next_id() -> usize {
        ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    pub fn new() -> Self {
        Self {
            id: Self::next_id(),
        }
    }
}
