use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use dam_core::{identifier::Identifier, log_graph::with_log_scope, time::Time, ContextView};
use petgraph::{dot::Dot, prelude::DiGraph};

use crate::{
    channel::{
        channel_spec::ChannelSpec,
        handle::{ChannelData, ChannelHandle},
        ChannelID, Receiver, Sender,
    },
    context::Context,
};

// A Program consists of all of its nodes and all of its edges.

#[derive(Default)]
pub struct Program<'a> {
    nodes: Vec<Box<dyn Context + 'a>>,
    // In order to perform optimizations such as flavor inference, the program also needs to hold onto all of its edges.
    edges: Vec<Arc<dyn ChannelHandle + 'a>>,
    void_edges: Vec<Arc<dyn ChannelHandle + 'a>>,
}

#[derive(Copy, Clone, Eq, Debug, PartialEq, Hash)]
enum ChannelOrContext {
    ChannelID(ChannelID),
    Context(Identifier),
}

impl<'a> Program<'a> {
    // Methods to add channels
    fn make_channel<T>(&mut self, capacity: Option<usize>) -> (Sender<T>, Receiver<T>)
    where
        T: Clone + 'a,
    {
        let spec = Arc::new(ChannelSpec::new(capacity));
        let underlying = Arc::new(ChannelData::new(spec));
        self.edges.push(underlying.clone());

        (
            Sender {
                underlying: underlying.clone(),
            },
            Receiver { underlying },
        )
    }

    pub fn bounded<T: Clone + 'a>(&mut self, capacity: usize) -> (Sender<T>, Receiver<T>) {
        self.make_channel(Some(capacity))
    }

    pub fn unbounded<T: Clone + 'a>(&mut self) -> (Sender<T>, Receiver<T>) {
        self.make_channel(None)
    }

    pub fn void<T: Clone + 'a>(&mut self) -> Sender<T> {
        let spec = Arc::new(ChannelSpec::new(None));
        let underlying = Arc::new(ChannelData::new(spec));
        self.void_edges.push(underlying.clone());
        Sender { underlying }
    }

    pub fn add_child<T>(&mut self, child: T)
    where
        T: Context + 'a,
    {
        self.nodes.push(Box::new(child));
    }

    fn all_node_ids(&self) -> HashSet<Identifier> {
        let tree = self.nodes.iter().map(|x| x.child_ids());
        HashSet::from_iter(
            tree.into_iter()
                .flat_map(|x| x.keys().copied().collect::<Vec<_>>()),
        )
    }

    pub fn init(&mut self) {
        // Make sure that all edges have registered endpoints.
        self.edges.iter().for_each(|edge| {
            assert!(edge.sender().is_some());
            assert!(edge.receiver().is_some());
        });

        self.void_edges.iter().for_each(|edge| {
            assert!(edge.sender().is_some());
            assert!(edge.receiver().is_none());
        });

        let all_node_ids = self.all_node_ids();
        // check that all of our edge targets are in the nodes
        self.edges.iter().chain(self.void_edges.iter()).for_each(|edge| {
            edge.sender()
                .iter()
                .chain(edge.receiver().iter())
                .for_each(|id| {
                    if !all_node_ids.contains(id) {
                        panic!("Node ID {id:?} is connected to an edge, but isn't registered to this program graph");
                    }
                })
        });

        // construct the edge reachability graph.
        // an edge is reachable from another edge

        self.void_edges
            .iter()
            .for_each(|edge| edge.set_flavor(crate::channel::ChannelFlavor::Void));

        let all_channel_ids: Vec<_> = self
            .edges
            .iter()
            .chain(self.void_edges.iter())
            .map(|handle| handle.id())
            .collect();

        let mut edge_graph = DiGraph::<ChannelOrContext, ()>::new();
        // All edges are nodes on the graph
        // all contexts map to one or more nodes
        let mut graph_node_map = HashMap::new();
        all_channel_ids.iter().for_each(|chan_id| {
            let handle = ChannelOrContext::ChannelID(*chan_id);
            let node = edge_graph.add_node(handle);
            graph_node_map.insert(handle, node);
        });

        let mut manually_managed_nodes = HashSet::new();

        for explicit_conn in self.nodes.iter().flat_map(|node| node.edge_connections()) {
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

        for node in all_node_ids {
            if !manually_managed_nodes.contains(&node) {
                let handle = ChannelOrContext::Context(node);
                graph_node_map.insert(handle, edge_graph.add_node(handle));
            }
        }

        // Now iterate over all the edges, populating the remaining stuff.
        for edge in self.edges.iter() {
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
        let actual_sccs: HashSet<_> =
            HashSet::from_iter(sccs.into_iter().filter(|x| x.len() > 1).flatten());

        // One of the major things to do here is to optimize all of the edges.
        self.edges.iter().for_each(|edge| {
            let handle = graph_node_map
                .get(&ChannelOrContext::ChannelID(edge.id()))
                .unwrap();
            if actual_sccs.contains(handle) {
                edge.set_flavor(crate::channel::ChannelFlavor::Cyclic);
            } else {
                edge.set_flavor(crate::channel::ChannelFlavor::Acyclic);
            }
        });

        self.nodes.iter_mut().for_each(|child| child.init());
    }

    pub fn run(&mut self) {
        std::thread::scope(|s| {
            self.nodes.iter_mut().for_each(|child| {
                let id = child.id();
                let name = child.name();
                std::thread::Builder::new()
                    .name(format!("{}({})", child.id(), child.name()))
                    .spawn_scoped(s, || {
                        with_log_scope(child.id(), child.name(), || {
                            child.run();
                            child.cleanup();
                        });
                    })
                    .unwrap_or_else(|_| panic!("Failed to spawn child {name:?} {id:?}"));
            });
        });
    }

    pub fn print_graph(&self) {
        let mut graph = DiGraph::<Identifier, ChannelID>::new();
        let ids = self.all_node_ids();
        let mut id_node_map = HashMap::new();
        for id in ids {
            id_node_map.insert(id, graph.add_node(id));
        }

        for edge in &self.edges {
            graph.add_edge(
                *id_node_map.get(&edge.sender().unwrap()).unwrap(),
                *id_node_map.get(&edge.receiver().unwrap()).unwrap(),
                edge.id(),
            );
        }

        println!("{:?}", Dot::with_config(&graph, &[]));
    }

    pub fn elapsed_cycles(&self) -> Time {
        let ticks = self
            .nodes
            .iter()
            .map(|child| child.view().tick_lower_bound());
        ticks.max().unwrap_or(Time::new(0))
    }
}
