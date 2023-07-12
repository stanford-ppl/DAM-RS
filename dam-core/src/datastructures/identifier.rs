use std::sync::atomic::AtomicUsize;

use serde::{Deserialize, Serialize};

use crate::log_graph::get_graph;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy, Serialize, Deserialize)]
pub struct Identifier {
    id: usize,
}
static COUNTER: AtomicUsize = AtomicUsize::new(0);
impl Identifier {
    pub fn new() -> Self {
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let res = Self { id };
        get_graph().add_id(res);
        res
    }
}

impl Default for Identifier {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ID_{}", self.id)
    }
}

pub trait Identifiable {
    fn id(&self) -> Identifier;

    fn name(&self) -> String;
}
