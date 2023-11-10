use crate::{
    datastructures::{Time, VerboseIdentifier},
    view::{ContextView, TimeView},
};

/// A basic summary of the execution of a context.
#[derive(Clone)]
pub struct ContextSummary {
    /// An identifier for the context
    pub id: VerboseIdentifier,

    /// A view of the time
    pub time: TimeView,

    /// A list of child context summaries -- this is needed because the top level program doesn't actually know about all of the nodes.
    pub children: Vec<ContextSummary>,
    // TODO: Should we include a field for a more complex summary from the node, or is it handled by logging?
}

impl ContextSummary {
    /// Gets the max time of the summary.
    /// This is probably overkill as the view itself should return the time.
    pub fn max_time(&self) -> u64 {
        [self.time.tick_lower_bound().time()]
            .into_iter()
            .chain(self.children.iter().map(|child| child.max_time()))
            .max()
            .unwrap()
    }
}
