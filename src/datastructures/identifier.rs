use std::sync::atomic::AtomicUsize;

use serde::{Deserialize, Serialize};

/// A guaranteed unique identifier for a context.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy, Serialize, Deserialize)]
pub struct Identifier {
    /// The actual ID
    pub id: usize,
}
static COUNTER: AtomicUsize = AtomicUsize::new(0);
impl Identifier {
    /// Obtains a new identifier by incrementing an atomic ID.
    pub fn new() -> Self {
        let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
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

/// A more complete identifier, replete with a name. This is mostly used to pass debug information around.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VerboseIdentifier {
    /// The underlying identifier
    pub id: Identifier,

    /// Some convenient name for debugging/visualization, usually the type of the Context.
    pub name: String,
}

/// A trait for mixing in identification methods, primarily used for macros.
pub trait Identifiable {
    /// Retrieves the identifier of the context.
    fn id(&self) -> Identifier;

    /// Gets the name of the context, usually the type name of the context.
    fn name(&self) -> String;

    /// Utility method to get both the id and the name, useful for debugging/logging.
    fn verbose(&self) -> VerboseIdentifier {
        VerboseIdentifier {
            id: self.id(),
            name: self.name(),
        }
    }
}
