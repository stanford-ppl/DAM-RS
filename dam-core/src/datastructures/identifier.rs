use std::sync::atomic::AtomicUsize;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Copy)]
pub struct Identifier {
    id: usize,
}

impl Identifier {
    const COUNTER: AtomicUsize = AtomicUsize::new(0);

    pub fn new() -> Self {
        Self {
            id: Identifier::COUNTER.fetch_add(1, std::sync::atomic::Ordering::AcqRel),
        }
    }
}

impl Default for Identifier {
    fn default() -> Self {
        Self::new()
    }
}

pub trait Identifiable {
    fn id(&self) -> Identifier;
}
