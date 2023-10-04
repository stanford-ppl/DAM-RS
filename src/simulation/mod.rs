mod building;
mod executed;
mod initialized;
mod programdata;

use crate::channel::ChannelID;
use dam_core::prelude::*;

#[derive(Debug, Default, Clone, Copy)]
pub enum RunMode {
    #[default]
    Simple,
    FIFO,
}

#[derive(Default)]
pub struct InitializationOptions {
    pub run_flavor_inference: bool,
}

// A Program consists of all of its nodes and all of its edges.

trait ProgramHelper {
    fn context_id_to_name(id: Identifier) -> String {
        format!("Node_{}", id.id)
    }
}

pub trait ProgramState {
    fn to_dot(&self) -> graphviz_rust::dot_structures::Graph;
}

#[derive(Error, Debug)]
pub enum InitializationError {
    #[error("Disconnected Sender on channel: {0:?}")]
    DisconnectedSender(ChannelID),

    #[error("Disconnected Receiver on channel: {0:?}")]
    DisconnectedReceiver(ChannelID),

    #[error("Unregistered Node: {0}")]
    UnregisteredNode(Identifier),
}

pub use building::ProgramBuilder;
pub use executed::Executed;
pub use initialized::Initialized;
use thiserror::Error;

// #[derive(Default)]
// pub struct Program<'a> {
//     nodes: Vec<Box<dyn Context + 'a>>,
//     // In order to perform optimizations such as flavor inference, the program also needs to hold onto all of its edges.
//     edges: Vec<Arc<dyn ChannelHandle + 'a>>,
//     void_edges: Vec<Arc<dyn ChannelHandle + 'a>>,

//     infer_flavors: bool,
//     run_mode: RunMode,
// }

// #[derive(Copy, Clone, Eq, Debug, PartialEq, Hash)]
// enum ChannelOrContext {
//     ChannelID(ChannelID),
//     Context(Identifier),
// }

// struct ChannelInfo {
//     pub id: ChannelID,
//     pub capacity: usize,
//     pub latency: u64,
// }

// impl ChannelInfo {
//     pub fn new(id: ChannelID, capacity: usize, latency: u64) -> ChannelInfo {
//         ChannelInfo {
//             id,
//             capacity,
//             latency,
//         }
//     }
// }

// impl fmt::Debug for ChannelInfo {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(
//             f,
//             "{:?} (Depth: {:?}, Latency: {:?})",
//             self.id, self.capacity, self.latency
//         )
//     }
// }

// impl<'a> Program<'a> {
//     // Methods to add channels
//     fn make_channel_with_latency<T>(
//         &mut self,
//         capacity: Option<usize>,
//         latency: Option<u64>,
//         resp_latency: Option<u64>,
//     ) -> (Sender<T>, Receiver<T>)
//     where
//         T: Clone + 'a,
//     {
//         let spec = Arc::new(ChannelSpec::new(capacity, latency, resp_latency));
//         let underlying = Arc::new(ChannelData::new(spec));
//         self.edges.push(underlying.clone());

//         (
//             Sender {
//                 underlying: underlying.clone(),
//             },
//             Receiver { underlying },
//         )
//     }

//     pub fn bounded<T: Clone + 'a>(&mut self, capacity: usize) -> (Sender<T>, Receiver<T>) {
//         self.make_channel_with_latency(Some(capacity), None, None)
//     }

//     pub fn bounded_with_latency<T: Clone + 'a>(
//         &mut self,
//         capacity: usize,
//         latency: u64,
//         resp_latency: u64,
//     ) -> (Sender<T>, Receiver<T>) {
//         self.make_channel_with_latency(Some(capacity), Some(latency), Some(resp_latency))
//     }

//     pub fn unbounded<T: Clone + 'a>(&mut self) -> (Sender<T>, Receiver<T>) {
//         self.make_channel_with_latency(None, None, None)
//     }

//     pub fn unbounded_with_latency<T: Clone + 'a>(
//         &mut self,
//         latency: u64,
//         resp_latency: u64,
//     ) -> (Sender<T>, Receiver<T>) {
//         self.make_channel_with_latency(None, Some(latency), Some(resp_latency))
//     }

//     pub fn void<T: Clone + 'a>(&mut self) -> Sender<T> {
//         let spec = Arc::new(ChannelSpec::new(None, None, None));
//         let underlying = Arc::new(ChannelData::new(spec));
//         self.void_edges.push(underlying.clone());
//         Sender { underlying }
//     }

//     pub fn add_child<T>(&mut self, child: T)
//     where
//         T: Context + 'a,
//     {
//         self.nodes.push(Box::new(child));
//     }

//     pub fn set_inference(&mut self, infer: bool) {
//         self.infer_flavors = infer;
//     }

//     pub fn set_mode(&mut self, mode: RunMode) {
//         self.run_mode = mode;
//     }

//     fn all_node_ids(&self) -> HashSet<Identifier> {
//         let tree = self.nodes.iter().map(|x| x.child_ids());
//         HashSet::from_iter(
//             tree.into_iter()
//                 .flat_map(|x| x.keys().copied().collect::<Vec<_>>()),
//         )
//     }

//     fn all_node_names(&self) -> HashMap<Identifier, String> {
//         let mut hashmap = HashMap::new();
//         for node in self.nodes.iter() {
//             hashmap.insert(node.id(), node.name());
//         }
//         hashmap
//     }

//     pub fn check(&self) {
//         // Make sure that all edges have registered endpoints.
//         self.edges.iter().for_each(|edge| {
//             if edge.sender().is_none() {
//                 panic!("Edge {:?} had no sender!", edge.id());
//             }
//             if edge.receiver().is_none() {
//                 panic!("Edge {:?} had no receiver!", edge.id());
//             }
//         });

//         self.void_edges.iter().for_each(|edge| {
//             if edge.sender().is_none() {
//                 panic!("Void edge {:?} had no sender!", edge.id());
//             }
//             if let Some(recv) = edge.receiver() {
//                 panic!("Void edge {:?} had a receiver! ({recv:?})", edge.id());
//             }
//         });

//         let all_node_ids = self.all_node_ids();
//         // check that all of our edge targets are in the nodes
//         self.edges.iter().chain(self.void_edges.iter()).for_each(|edge| {
//             edge.sender()
//                 .iter()
//                 .chain(edge.receiver().iter())
//                 .for_each(|id| {
//                     if !all_node_ids.contains(id) {
//                         panic!("Node ID {id:?} is connected to an edge, but isn't registered to this program graph");
//                     }
//                 })
//         });
//     }

//     pub fn init(&mut self) {
//         // construct the edge reachability graph.
//         // an edge is reachable from another edge
//         self.check();

//         self.void_edges
//             .iter()
//             .for_each(|edge| edge.set_flavor(crate::channel::ChannelFlavor::Void));

//         if self.infer_flavors {
//             let all_channel_ids: Vec<_> = self
//                 .edges
//                 .iter()
//                 .chain(self.void_edges.iter())
//                 .map(|handle| handle.id())
//                 .collect();

//             let mut edge_graph = DiGraph::<ChannelOrContext, ()>::new();
//             // All edges are nodes on the graph
//             // all contexts map to one or more nodes
//             let mut graph_node_map = FxHashMap::default();
//             all_channel_ids.iter().for_each(|chan_id| {
//                 let handle = ChannelOrContext::ChannelID(*chan_id);
//                 let node = edge_graph.add_node(handle);
//                 graph_node_map.insert(handle, node);
//             });

//             let mut manually_managed_nodes = FxHashSet::default();

//             for explicit_conn in self.nodes.iter().flat_map(|node| node.edge_connections()) {
//                 for (node, mapping) in explicit_conn {
//                     manually_managed_nodes.insert(node);
//                     for (srcs, dsts) in mapping {
//                         let temp_node = edge_graph.add_node(ChannelOrContext::Context(node));
//                         for src in srcs {
//                             edge_graph.add_edge(
//                                 *graph_node_map
//                                     .get(&ChannelOrContext::ChannelID(src))
//                                     .unwrap(),
//                                 temp_node,
//                                 (),
//                             );
//                         }

//                         for dst in dsts {
//                             edge_graph.add_edge(
//                                 temp_node,
//                                 *graph_node_map
//                                     .get(&ChannelOrContext::ChannelID(dst))
//                                     .unwrap(),
//                                 (),
//                             );
//                         }
//                     }
//                 }
//             }

//             for node in self.all_node_ids() {
//                 if !manually_managed_nodes.contains(&node) {
//                     let handle = ChannelOrContext::Context(node);
//                     graph_node_map.insert(handle, edge_graph.add_node(handle));
//                 }
//             }

//             // Now iterate over all the edges, populating the remaining stuff.
//             for edge in self.edges.iter() {
//                 let own_node = graph_node_map
//                     .get(&ChannelOrContext::ChannelID(edge.id()))
//                     .unwrap();
//                 let src = edge.sender().unwrap();
//                 if !manually_managed_nodes.contains(&src) {
//                     // connect the source onto ourselves
//                     edge_graph.add_edge(
//                         *graph_node_map.get(&ChannelOrContext::Context(src)).unwrap(),
//                         *own_node,
//                         (),
//                     );
//                 }

//                 let dst = edge.receiver().unwrap();
//                 if !manually_managed_nodes.contains(&dst) {
//                     edge_graph.add_edge(
//                         *own_node,
//                         *graph_node_map.get(&ChannelOrContext::Context(dst)).unwrap(),
//                         (),
//                     );
//                 }
//             }

//             let sccs = petgraph::algo::tarjan_scc(&edge_graph);
//             let actual_sccs: HashSet<_> =
//                 HashSet::from_iter(sccs.into_iter().filter(|x| x.len() > 1).flatten());

//             // One of the major things to do here is to optimize all of the edges.
//             self.edges.iter().for_each(|edge| {
//                 let handle = graph_node_map
//                     .get(&ChannelOrContext::ChannelID(edge.id()))
//                     .unwrap();
//                 if actual_sccs.contains(handle) {
//                     edge.set_flavor(crate::channel::ChannelFlavor::Cyclic);
//                 } else {
//                     edge.set_flavor(crate::channel::ChannelFlavor::Acyclic);
//                 }
//             });
//         } else {
//             self.edges
//                 .iter()
//                 .for_each(|edge| edge.set_flavor(crate::channel::ChannelFlavor::Cyclic));
//         }

//         self.nodes.iter_mut().for_each(|child| child.init());
//     }

//     pub fn run(&mut self) {
//         let (priority, policy) = match self.run_mode {
//             RunMode::Simple => (
//                 thread_priority::get_current_thread_priority().unwrap(),
//                 thread_priority::thread_schedule_policy().unwrap(),
//             ),
//             RunMode::FIFO => {
//                 let priority =
//                     thread_priority::ThreadPriority::Crossplatform(10u8.try_into().unwrap());
//                 let policy = thread_priority::unix::ThreadSchedulePolicy::Realtime(
//                     thread_priority::RealtimeThreadSchedulePolicy::Fifo,
//                 );
//                 (priority, policy)
//             }
//         };

//         std::thread::scope(|s| {
//             std::mem::take(&mut self.nodes)
//                 .into_iter()
//                 .for_each(|mut child| {
//                     let id = child.id();
//                     let name = child.name();
//                     let builder = thread_priority::ThreadBuilder::default().name(format!(
//                         "{}({})",
//                         child.id(),
//                         child.name()
//                     ));

//                     let builder = builder.priority(priority).policy(policy);

//                     builder
//                         .spawn_scoped_careless(s, move || {
//                             child.run();
//                             println!("Child Finished: {:?} {:?}", child.id(), child.name());
//                         })
//                         .unwrap_or_else(|_| panic!("Failed to spawn child {name:?} {id:?}"));
//                 });
//         });
//     }

//     pub fn print_graph(&self) {
//         self.check();
//         let mut graph = DiGraph::<Identifier, ChannelID>::new();
//         let ids = self.all_node_ids();
//         let mut id_node_map = HashMap::new();
//         for id in ids {
//             id_node_map.insert(id, graph.add_node(id));
//         }

//         for edge in &self.edges {
//             graph.add_edge(
//                 *id_node_map
//                     .get(&edge.sender().expect("Edge didn't have a sender!"))
//                     .expect("Edge sender was not registered in id_node_map!"),
//                 *id_node_map
//                     .get(&edge.receiver().expect("Edge didn't have a receiver!"))
//                     .expect("Edge receiver was not registered in id_node_map!"),
//                 edge.id(),
//             );
//         }

//         println!("{:?}", Dot::with_config(&graph, &[]));
//     }

//     pub fn print_graph_with_names(&self) {
//         self.check();
//         let mut graph = DiGraph::<&str, ChannelInfo>::new();
//         let ids = self.all_node_ids();
//         let node_names = self.all_node_names();
//         let mut id_node_map: HashMap<Identifier, petgraph::stable_graph::NodeIndex> =
//             HashMap::new();
//         for id in ids {
//             id_node_map.insert(id, graph.add_node(node_names[&id].as_str().clone()));
//         }

//         for edge in &self.edges {
//             graph.add_edge(
//                 *id_node_map
//                     .get(&edge.sender().expect("Edge didn't have a sender!"))
//                     .expect("Edge sender was not registered in id_node_map!"),
//                 *id_node_map
//                     .get(&edge.receiver().expect("Edge didn't have a receiver!"))
//                     .expect("Edge receiver was not registered in id_node_map!"),
//                 ChannelInfo::new(
//                     edge.id(),
//                     edge.spec().capacity().unwrap(),
//                     edge.spec().latency(),
//                 ),
//             );
//         }

//         println!("{:?}", Dot::with_config(&graph, &[]));
//     }

//     pub fn elapsed_cycles(&self) -> Time {
//         let ticks = self
//             .nodes
//             .iter()
//             .map(|child| child.view().tick_lower_bound());
//         ticks.max().unwrap_or(Time::new(0))
//     }
// }
