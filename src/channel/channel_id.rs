use std::sync::atomic::AtomicUsize;

use serde::{Deserialize, Serialize};

static ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A unique identifier for a channel. Not guaranteed stable across program runs.
#[derive(Serialize, Deserialize, Debug, Copy, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct ChannelID {
    id: usize,
}

impl ChannelID {
    fn next_id() -> usize {
        ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// Construct a new ChannelID
    pub fn new() -> Self {
        Self {
            id: Self::next_id(),
        }
    }
}

impl Default for ChannelID {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ChannelID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Channel({})", self.id)
    }
}
