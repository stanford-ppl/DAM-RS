use crate::view::TimeManager;

mod identifier;

/// Get rid of this after <https://github.com/rust-lang/rust/issues/95439> is resolved.
pub(crate) mod sync_unsafe;

mod time;

pub use identifier::*;
pub(crate) use time::AtomicTime;
pub use time::Time;

mod marker;
pub use marker::*;

/// A utility construct for grouping together time and an ID for a context. This is used via Deref<Target=ContextInfo> for Context.
/// As a result, users can do self.time, self.id, and access other members that the DAM team would like to add without needing to augment the constructors.
#[derive(Default, Debug)]
pub struct ContextInfo {
    /// The time of the context
    pub time: TimeManager,

    /// The context's identifier
    pub id: identifier::Identifier,
}
