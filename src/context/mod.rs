//! Context provides the basic traits and wrappers for defining the behavior of logical units.
//! Each context is programmed in a CSP-like fashion, expressing its entire execution via a monolithic [Context::run] method, which accepts arbitrary user code.

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

/// Explicitly specified connections from input channels to output channels -- used for arbiter-like contexts.
pub type ExplicitConnections = HashMap<Identifier, Vec<(HashSet<ChannelID>, HashSet<ChannelID>)>>;

/// The core component of DAM.
pub trait Context: Send + Sync + TimeViewable + Identifiable {
    // A lot of contexts simply define this to be empty anyways.
    /// Initializes the context -- frequently a no-op. No guarantee that initialization is executed in the same thread as the run operation currently.
    fn init(&mut self) {}

    /// The 'meat-and-bones' of a context, expressed as a monolithic function.
    fn run(&mut self);

    /// A map of IDs to child IDs. For most nodes this will be ID -> Empty.
    fn ids(&self) -> HashMap<VerboseIdentifier, HashSet<VerboseIdentifier>> {
        HashMap::from([(self.verbose(), HashSet::new())])
    }

    /// By default all edges are connected.
    /// In the case of something like a PMU, however, we wish to be finer-grained than that.
    /// In that case, we can report channel A -> {B, C, D} means that A sends data that can be observed on B, C, and/or D.
    fn edge_connections(&self) -> Option<ExplicitConnections> {
        None
    }

    /// Returns a summary of the context, which is then dropped by the programgraph.
    fn summarize(&self) -> ContextSummary {
        ContextSummary {
            id: self.verbose(),
            time: self.view(),
            children: vec![],
        }
    }
}
