use crate::time::Time;

use super::{ContextView, TimeView};

pub struct ParentView {
    pub child_views: Vec<TimeView>,
}

impl ContextView for ParentView {
    fn wait_until(&self, when: Time) -> Time {
        let individual_signals: Vec<_> = self
            .child_views
            .iter()
            .map(|child| child.wait_until(when))
            .collect();
        individual_signals.into_iter().min().unwrap_or(when)
    }

    fn tick_lower_bound(&self) -> Time {
        let min_time = self
            .child_views
            .iter()
            .map(|child| child.tick_lower_bound())
            .min();
        match min_time {
            Some(time) => time,
            None => Time::infinite(),
        }
    }
}
