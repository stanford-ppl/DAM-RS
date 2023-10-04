use crate::view::TimeManager;

mod identifier;
pub mod sync_unsafe;
mod time;
mod unrolled_linked_list;

pub use identifier::*;
pub use time::AtomicTime;
pub use time::Time;

#[derive(Default, Debug)]
pub struct ContextInfo {
    pub time: TimeManager,
    pub id: identifier::Identifier,
}
