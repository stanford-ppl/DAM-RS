use std::collections::{HashMap, HashSet};

use crate::{
    channel::ChannelID,
    datastructures::{Identifiable, Identifier, VerboseIdentifier},
    view::TimeViewable,
};

mod proxy;

mod summary;
pub use summary::ContextSummary;

pub use proxy::ProxyContext;

pub type ExplicitConnections = HashMap<Identifier, Vec<(HashSet<ChannelID>, HashSet<ChannelID>)>>;

pub trait Context: Send + Sync + TimeViewable + Identifiable {
    // A lot of contexts simply define this to be empty anyways.
    fn init(&mut self) {}
    fn run(&mut self);

    fn ids(&self) -> HashMap<VerboseIdentifier, HashSet<VerboseIdentifier>> {
        HashMap::from([(self.verbose(), HashSet::new())])
    }

    // By default all edges are connected.
    // In the case of something like a PMU, however, we wish to be finer-grained than that.
    // In that case, we can report channel A -> {B, C, D} means that A sends data that can be observed on B, C, and/or D.
    fn edge_connections(&self) -> Option<ExplicitConnections> {
        None
    }

    fn summarize(&self) -> ContextSummary {
        ContextSummary {
            id: self.verbose(),
            time: self.view(),
            children: vec![],
        }
    }
}
