use std::collections::{HashMap, HashSet};

use dam_core::{
    identifier::{Identifiable, Identifier},
    TimeViewable,
};

use crate::channel::ChannelID;

pub mod approx_checker_context;
pub mod broadcast_context;
pub mod checker_context;
pub mod function_context;
pub mod generator_context;
pub mod parent;
pub mod print_context;

pub type ExplicitConnections = HashMap<Identifier, Vec<(HashSet<ChannelID>, HashSet<ChannelID>)>>;

pub trait Context: Send + Sync + TimeViewable + Identifiable {
    fn init(&mut self);
    fn run(&mut self);
    fn cleanup(&mut self);

    fn child_ids(&self) -> HashMap<Identifier, HashSet<Identifier>> {
        HashMap::from([(self.id(), HashSet::new())])
    }

    fn child_names(&self) -> HashMap<Identifier, String> {
        HashMap::from([(self.id(), self.name())])
    }

    // By default all edges are connected.
    // In the case of something like a PMU, however, we wish to be finer-grained than that.
    // In that case, we can report channel A -> {B, C, D} means that A sends data that can be observed on B, C, and/or D.
    fn edge_connections(&self) -> Option<ExplicitConnections> {
        None
    }
}
