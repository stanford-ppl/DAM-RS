use std::sync::Arc;

use dam_core::prelude::*;

use crate::{channel::handle::ChannelHandle, context::ContextSummary};

pub struct Executed<'a> {
    pub(super) nodes: Vec<ContextSummary>,
    pub(super) edges: Vec<Arc<dyn ChannelHandle + 'a>>,
}

impl Executed<'_> {
    pub fn elapsed_cycles(&self) -> Option<Time> {
        self.nodes.iter().map(|node| node.max_time()).max()
    }
}
