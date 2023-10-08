use crate::{
    datastructures::{Time, VerboseIdentifier},
    view::{ContextView, TimeView},
};

#[derive(Clone)]
pub struct ContextSummary {
    pub id: VerboseIdentifier,
    pub time: TimeView,
    pub children: Vec<ContextSummary>,
}

impl ContextSummary {
    pub fn max_time(&self) -> Time {
        [self.time.tick_lower_bound()]
            .into_iter()
            .chain(self.children.iter().map(|child| child.max_time()))
            .max()
            .unwrap()
    }
}
