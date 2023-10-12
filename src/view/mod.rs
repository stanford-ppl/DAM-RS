mod basic;
mod parent;

pub use basic::BasicContextView;
pub use basic::TimeManager;
pub use parent::ParentView;

use crate::datastructures::Time;

/// Enables viewing a context.
#[enum_delegate::register]
pub trait ContextView {
    /// Blocks the waiting context until the viewed context reaches a certain time.
    /// This may or may not actually block, depending on whether the viewed context has already progressed.
    fn wait_until(&self, when: Time) -> Time;

    /// Reads the time of the viewed context.
    /// This is only guaranteed to be a lower bound, as the viewed context may have progressed since the write.
    fn tick_lower_bound(&self) -> Time;
}

/// enum_delegate enum to change users from `Box<dyn ContextView>`.
#[enum_delegate::implement(ContextView)]
#[derive(Clone)]
pub enum TimeView {
    /// See [BasicContextView]
    BasicContextView(BasicContextView),

    /// See [ParentView]
    ParentView(ParentView),
}

/// Structures which may be viewed.
/// Used to parcel out the implementation to help macro-driven implementation.
/// This should only be used when implementing contexts.
pub trait TimeViewable {
    /// Obtain a view of the context.
    fn view(&self) -> TimeView;
}
