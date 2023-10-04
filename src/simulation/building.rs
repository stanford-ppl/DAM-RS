use std::sync::Arc;

use dam_core::prelude::*;
use petgraph::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
    channel::{
        channel_spec::ChannelSpec,
        handle::{ChannelData, ChannelHandle},
        ChannelID, Receiver, Sender,
    },
    context::Context,
};

use super::{
    programdata::ProgramData, InitializationError, InitializationOptions, Initialized,
};

#[derive(Copy, Clone, Eq, Debug, PartialEq, Hash)]
enum ChannelOrContext {
    ChannelID(ChannelID),
    Context(Identifier),
}

#[derive(Default)]
pub struct ProgramBuilder<'a> {
    data: ProgramData<'a>,
}
impl<'a> ProgramBuilder<'a> {
    fn add_edge(&mut self, edge: Arc<dyn ChannelHandle + 'a>) {
        self.data.edges.push(edge);
    }

    fn add_void_edge(&mut self, edge: Arc<dyn ChannelHandle + 'a>) {
        self.data.void_edges.push(edge);
    }

    fn add_node(&mut self, node: Box<dyn Context + 'a>) {
        self.data.nodes.push(node);
    }

    fn make_channel_with_latency<T>(
        &mut self,
        capacity: Option<usize>,
        latency: Option<u64>,
        resp_latency: Option<u64>,
    ) -> (Sender<T>, Receiver<T>)
    where
        T: Clone + 'a,
    {
        let spec = Arc::new(ChannelSpec::new(capacity, latency, resp_latency));
        let underlying = Arc::new(ChannelData::new(spec));
        self.add_edge(underlying.clone());

        (
            Sender {
                underlying: underlying.clone(),
            },
            Receiver { underlying },
        )
    }

    pub fn bounded<T: Clone + 'a>(&mut self, capacity: usize) -> (Sender<T>, Receiver<T>) {
        self.make_channel_with_latency(Some(capacity), None, None)
    }

    pub fn bounded_with_latency<T: Clone + 'a>(
        &mut self,
        capacity: usize,
        latency: u64,
        resp_latency: u64,
    ) -> (Sender<T>, Receiver<T>) {
        self.make_channel_with_latency(Some(capacity), Some(latency), Some(resp_latency))
    }

    pub fn unbounded<T: Clone + 'a>(&mut self) -> (Sender<T>, Receiver<T>) {
        self.make_channel_with_latency(None, None, None)
    }

    pub fn unbounded_with_latency<T: Clone + 'a>(
        &mut self,
        latency: u64,
        resp_latency: u64,
    ) -> (Sender<T>, Receiver<T>) {
        self.make_channel_with_latency(None, Some(latency), Some(resp_latency))
    }

    pub fn void<T: Clone + 'a>(&mut self) -> Sender<T> {
        let spec = Arc::new(ChannelSpec::new(None, None, None));
        let underlying = Arc::new(ChannelData::new(spec));
        self.add_void_edge(underlying.clone());
        Sender { underlying }
    }

    pub fn add_child<T>(&mut self, child: T)
    where
        T: Context + 'a,
    {
        self.add_node(Box::new(child));
    }

    pub fn initialize(
        mut self,
        options: InitializationOptions,
    ) -> Result<Initialized<'a>, InitializationError> {
        self.data.check()?;
        self.data
            .void_edges
            .iter()
            .for_each(|edge| edge.set_flavor(crate::channel::ChannelFlavor::Void));

        if options.run_flavor_inference {
            let all_channel_ids: Vec<_> = self
                .data
                .edges
                .iter()
                .chain(self.data.void_edges.iter())
                .map(|handle| handle.id())
                .collect();

            let mut edge_graph = DiGraph::<ChannelOrContext, ()>::new();
            // All edges are nodes on the graph
            // all contexts map to one or more nodes
            let mut graph_node_map = FxHashMap::default();
            all_channel_ids.iter().for_each(|chan_id| {
                let handle = ChannelOrContext::ChannelID(*chan_id);
                let node = edge_graph.add_node(handle);
                graph_node_map.insert(handle, node);
            });

            let mut manually_managed_nodes = FxHashSet::default();

            for explicit_conn in self
                .data
                .nodes
                .iter()
                .flat_map(|node| node.edge_connections())
            {
                for (node, mapping) in explicit_conn {
                    manually_managed_nodes.insert(node);
                    for (srcs, dsts) in mapping {
                        let temp_node = edge_graph.add_node(ChannelOrContext::Context(node));
                        for src in srcs {
                            edge_graph.add_edge(
                                *graph_node_map
                                    .get(&ChannelOrContext::ChannelID(src))
                                    .unwrap(),
                                temp_node,
                                (),
                            );
                        }

                        for dst in dsts {
                            edge_graph.add_edge(
                                temp_node,
                                *graph_node_map
                                    .get(&ChannelOrContext::ChannelID(dst))
                                    .unwrap(),
                                (),
                            );
                        }
                    }
                }
            }

            for (node, _) in self.data.node_identifiers() {
                if !manually_managed_nodes.contains(&node) {
                    let handle = ChannelOrContext::Context(node);
                    graph_node_map.insert(handle, edge_graph.add_node(handle));
                }
            }

            // Now iterate over all the edges, populating the remaining stuff.
            for edge in self.data.edges.iter() {
                let own_node = graph_node_map
                    .get(&ChannelOrContext::ChannelID(edge.id()))
                    .unwrap();
                let src = edge.sender().unwrap();
                if !manually_managed_nodes.contains(&src) {
                    // connect the source onto ourselves
                    edge_graph.add_edge(
                        *graph_node_map.get(&ChannelOrContext::Context(src)).unwrap(),
                        *own_node,
                        (),
                    );
                }

                let dst = edge.receiver().unwrap();
                if !manually_managed_nodes.contains(&dst) {
                    edge_graph.add_edge(
                        *own_node,
                        *graph_node_map.get(&ChannelOrContext::Context(dst)).unwrap(),
                        (),
                    );
                }
            }

            let sccs = petgraph::algo::tarjan_scc(&edge_graph);
            let actual_sccs: FxHashSet<_> =
                sccs.into_iter().filter(|x| x.len() > 1).flatten().collect();

            // One of the major things to do here is to optimize all of the edges.
            self.data.edges.iter().for_each(|edge| {
                let handle = graph_node_map
                    .get(&ChannelOrContext::ChannelID(edge.id()))
                    .unwrap();
                if actual_sccs.contains(handle) {
                    edge.set_flavor(crate::channel::ChannelFlavor::Cyclic);
                } else {
                    edge.set_flavor(crate::channel::ChannelFlavor::Acyclic);
                }
            });
        } else {
            self.data
                .edges
                .iter()
                .for_each(|edge| edge.set_flavor(crate::channel::ChannelFlavor::Cyclic));
        }

        self.data.nodes.iter_mut().for_each(|child| child.init());

        Ok(Initialized { data: self.data })
    }
}
