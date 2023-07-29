use core::fmt;
use std::sync::atomic::AtomicUsize;

use serde::{Deserialize, Serialize};

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);
#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
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

impl fmt::Debug for ChannelID {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Channel {:?}", self.id)
    }
}

impl Default for ChannelID {
    fn default() -> Self {
        Self::new()
    }
}
