use std::sync::atomic::AtomicUsize;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy, Serialize, Deserialize)]
pub struct Identifier {
    id: usize,
}
static COUNTER: AtomicUsize = AtomicUsize::new(0);
impl Identifier {
    pub fn new() -> Self {
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        Self { id }
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VerboseIdentifier {
    pub id: Identifier,
    pub name: String,
}

pub trait Identifiable {
    fn id(&self) -> Identifier;

    fn name(&self) -> String;

    fn verbose(&self) -> VerboseIdentifier {
        VerboseIdentifier {
            id: self.id(),
            name: self.name(),
        }
    }
}
