use std::{collections::HashSet, sync::Arc};

use dam_core::{identifier::Identifier, log_graph::with_log_scope};

use crate::{
    channel::{
        channel_spec::ChannelSpec,
        handle::{ChannelData, ChannelHandle},
        Receiver, Sender,
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
            Receiver {
                underlying: underlying.clone(),
            },
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

    pub fn init(&mut self) {
        let all_node_ids: HashSet<Identifier> = HashSet::from_iter(
            self.nodes.iter().map(|node| node.id()).chain(
                self.nodes
                    .iter()
                    .flat_map(|node| node.child_ids().into_iter()),
            ),
        );

        // Make sure that all edges have registered endpoints.
        self.edges.iter().for_each(|edge| {
            assert!(edge.sender().is_some());
            assert!(edge.receiver().is_some());
        });

        self.void_edges.iter().for_each(|edge| {
            assert!(edge.sender().is_some());
            assert!(edge.receiver().is_none());
        });

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

        // One of the major things to do here is to optimize all of the edges.
        self.edges
            .iter()
            .for_each(|edge| edge.set_flavor(crate::channel::ChannelFlavor::Cyclic));

        self.void_edges
            .iter()
            .for_each(|edge| edge.set_flavor(crate::channel::ChannelFlavor::Void));
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
                    .expect(format!("Failed to spawn child {name:?} {id:?}").as_str());
            });
        });
    }
}
