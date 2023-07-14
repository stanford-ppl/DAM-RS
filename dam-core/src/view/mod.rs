mod basic;
mod parent;

pub use basic::BasicContextView;
pub use basic::TimeManager;
pub use parent::ParentView;

use crate::datastructures::time::Time;
#[enum_delegate::register]
pub trait ContextView {
    fn wait_until(&self, when: Time) -> Time;
    fn tick_lower_bound(&self) -> Time;
}

#[enum_delegate::implement(ContextView)]
pub enum TimeView {
    BasicContextView(BasicContextView),
    ParentView(ParentView),
}

pub trait TimeViewable {
    fn view(&self) -> TimeView;
}

pub trait TimeManaged {
    fn time_manager_mut(&mut self) -> &mut TimeManager;
    fn time_manager(&self) -> &TimeManager;
}
