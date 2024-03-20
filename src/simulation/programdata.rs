use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::{channel::handle::ChannelHandle, context::Context, datastructures::Identifier};

use super::InitializationError;

#[derive(Default)]
pub(super) struct ProgramData<'a> {
    pub(super) nodes: Vec<Box<dyn Context + 'a>>,
    pub(super) edges: Vec<Arc<dyn ChannelHandle + 'a>>,
    pub(super) void_edges: Vec<Arc<dyn ChannelHandle + 'a>>,
}

impl ProgramData<'_> {
    pub(super) fn node_identifiers(&self) -> FxHashMap<Identifier, String> {
        self.nodes
            .iter()
            .flat_map(|node| node.ids())
            .map(|(verbose, _)| (verbose.id, verbose.name))
            .collect()
    }

    pub(super) fn check(&self) -> Result<(), InitializationError> {
        // Make sure that all edges have registered endpoints.
        for edge in &self.edges {
            if edge.sender().is_none() {
                return Err(InitializationError::DisconnectedSender(edge.id()));
            }
            if edge.receiver().is_none() {
                return Err(InitializationError::DisconnectedReceiver(edge.id()));
            }
        }

        for edge in &self.void_edges {
            if edge.sender().is_none() {
                return Err(InitializationError::DisconnectedSender(edge.id()));
            }
            if let Some(recv) = edge.receiver() {
                // This is a panic because it should NEVER happen.
                panic!("Void edge {:?} had a receiver! ({recv:?})", edge.id());
            }
        }

        let all_node_ids = self.node_identifiers();
        // check that all of our edge targets are in the nodes
        for edge in self.edges.iter().chain(self.void_edges.iter()) {
            for id in edge.sender().iter().chain(edge.receiver().iter()) {
                if !all_node_ids.contains_key(id) {
                    return Err(InitializationError::UnregisteredNode(*id));
                }
            }
        }

        Ok(())
    }
}
