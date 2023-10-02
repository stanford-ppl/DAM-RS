mod basic;
mod parent;

pub use basic::BasicContextView;
pub use basic::TimeManager;
pub use parent::ParentView;

use crate::datastructures::Time;

#[enum_delegate::register]
pub trait ContextView {
    fn wait_until(&self, when: Time) -> Time;
    fn tick_lower_bound(&self) -> Time;
}

#[enum_delegate::implement(ContextView)]
#[derive(Clone)]
pub enum TimeView {
    BasicContextView(BasicContextView),
    ParentView(ParentView),
}

pub trait TimeViewable {
    fn view(&self) -> TimeView;
}
