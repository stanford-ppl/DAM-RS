use std::{
    default,
    sync::{Arc, Mutex, RwLock},
};

use crossbeam::queue::SegQueue;
use dam_core::logging::{initialize_log, Logger};

use super::{executed::Executed, programdata::ProgramData, RunMode, RunOptions};

pub struct Initialized<'a> {
    pub(super) data: ProgramData<'a>,
}

impl<'a> Initialized<'a> {
    pub fn run(mut self, options: RunOptions) -> Executed<'a> {
        let (priority, policy) = match options.mode {
            RunMode::Simple => (
                thread_priority::get_current_thread_priority().unwrap(),
                thread_priority::thread_schedule_policy().unwrap(),
            ),
            RunMode::FIFO => {
                let priority =
                    thread_priority::ThreadPriority::Crossplatform(10u8.try_into().unwrap());
                let policy = thread_priority::unix::ThreadSchedulePolicy::Realtime(
                    thread_priority::RealtimeThreadSchedulePolicy::Fifo,
                );
                (priority, policy)
            }
        };

        let runtime = match options.logging {
            super::LoggingOptions::None => None,
            super::LoggingOptions::Mongodb(_) => Some(Arc::new(
                tokio::runtime::Builder::new_current_thread()
                    .worker_threads(2)
                    .on_thread_start(move || {
                        thread_priority::set_current_thread_priority(priority)
                            .expect("Tokio worker thread priority failed!");
                    })
                    .build()
                    .unwrap(),
            )),
        };

        let summaries = Arc::new(SegQueue::new());

        let start_time = std::time::Instant::now();

        std::thread::scope(|s| {
            let builder = thread_priority::ThreadBuilder::default()
                .priority(priority)
                .policy(policy);

            self.data.nodes.drain(..).for_each(|mut child| {
                let id = child.id();
                let name = child.name();
                let builder = builder
                    .clone()
                    .name(format!("{}({})", child.id(), child.name()));
                let summary_queue = summaries.clone();

                let rt_clone = runtime.clone();
                let log_options = options.logging.clone();
                builder
                    .spawn_scoped_careless(s, move || {
                        match log_options {
                            super::LoggingOptions::None => {}
                            super::LoggingOptions::Mongodb(mongo) => initialize_log(Logger::new(
                                rt_clone.unwrap(),
                                start_time,
                                mongo,
                                child.id(),
                            )),
                        }
                        child.run();
                        summary_queue.push(child.summarize());
                    })
                    .unwrap_or_else(|_| panic!("Failed to spawn child {name:?} {id:?}"));
            });
        });
        Executed {
            nodes: Arc::into_inner(summaries).unwrap().into_iter().collect(),
            edges: self.data.edges,
        }
    }
}

#[cfg(feature = "dot")]
mod inner {
    use std::collections::HashMap;
    use std::collections::HashSet;

    use dam_core::prelude::Identifier;
    use dam_core::prelude::VerboseIdentifier;
    use graphviz_rust::dot_generator::*;
    use graphviz_rust::dot_structures::*;
    use rustc_hash::FxHashSet;

    use crate::simulation::dot::DotConvertibleHelper;

    impl super::Initialized<'_> {
        fn emit_node(
            node: &VerboseIdentifier,
            visited: &mut FxHashSet<Identifier>,
            node_graph: &HashMap<VerboseIdentifier, HashSet<VerboseIdentifier>>,
        ) -> Vec<Stmt> {
            let mut result = vec![];
            if visited.contains(&node.id) {
                return result;
            }
            visited.insert(node.id);

            let label_string = format!("{}({})", node.name, node.id);
            let children = &node_graph[node];

            // Leaf node
            if children.is_empty() {
                result.push(
                    Node::new(
                        node_id!(Self::context_id_to_name(node.id)),
                        vec![
                            attr!("shape", esc "rectangle"),
                            attr!("label", esc label_string),
                        ],
                    )
                    .into(),
                );
            } else {
                let mut inner_stmts = vec![stmt!(attr!("label", esc label_string))];

                for child in children {
                    inner_stmts.extend(Self::emit_node(child, visited, node_graph))
                }

                result.push(
                    Subgraph {
                        id: Id::Plain(format!("cluster_{}", Self::context_id_to_name(node.id))),
                        stmts: inner_stmts,
                    }
                    .into(),
                );
            }
            result
        }
    }

    impl DotConvertibleHelper for super::Initialized<'_> {
        fn add_nodes(&self) -> Vec<Stmt> {
            let node_graph: HashMap<_, _> = self
                .data
                .nodes
                .iter()
                .map(|node| node.ids())
                .flatten()
                .collect();

            let mut stmts = vec![];
            let mut visited = FxHashSet::default();
            for node in &self.data.nodes {
                stmts.extend(Self::emit_node(&node.verbose(), &mut visited, &node_graph))
            }
            stmts
        }

        fn generate_edges(&self) -> Vec<graphviz_rust::dot_structures::Stmt> {
            self.data
                .edges
                .iter()
                .map(|edge| Self::generate_edge(edge.clone()))
                .flatten()
                .collect()
        }
    }
}
