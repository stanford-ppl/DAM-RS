use std::sync::Arc;

use crate::logging::{initialize_log, LogEntry, LogInterface, LogProcessor};
use crossbeam::queue::SegQueue;

#[cfg(feature = "log-mongo")]
use crate::logging::{mongodb, MongoLogger};

use super::{executed::Executed, programdata::ProgramData, LoggingOptions, RunMode, RunOptions};

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

        let (log_sender, log_receiver) = crossbeam::channel::unbounded();

        let exec_logger =
            Self::make_logger(log_receiver, options.logging).expect("Error creating logger!");

        let has_logger = exec_logger.is_some();

        let summaries = Arc::new(SegQueue::new());

        std::thread::scope(|s| {
            let builder = thread_priority::ThreadBuilder::default()
                .priority(priority)
                .policy(policy);

            let base_time = std::time::Instant::now();

            self.data.nodes.drain(..).for_each(|mut child| {
                let id = child.id();
                let name = child.name();
                let builder = builder
                    .clone()
                    .name(format!("{}({})", child.id(), child.name()));
                let summary_queue = summaries.clone();
                let filter_copy = options.log_filter.clone();

                let sender = log_sender.clone();
                builder
                    .spawn_scoped_careless(s, move || {
                        if has_logger {
                            let active_filter = match filter_copy {
                                super::LogFilterKind::Blanket(filter) => filter,
                                super::LogFilterKind::PerChild(func) => func(child.id()),
                            };
                            initialize_log(LogInterface::new(
                                child.id(),
                                sender,
                                base_time,
                                active_filter,
                            ));
                        }
                        child.run();
                        summary_queue.push(child.summarize());
                    })
                    .unwrap_or_else(|_| panic!("Failed to spawn child {name:?} {id:?}"));
            });

            drop(log_sender);

            if let Some(mut logger) = exec_logger {
                builder
                    .spawn_scoped_careless(s, move || logger.spawn())
                    .unwrap_or_else(|_| panic!("Failed to start logging thread!"));
            }
        });

        Executed {
            nodes: Arc::into_inner(summaries).unwrap().into_iter().collect(),
            edges: self.data.edges,
        }
    }

    // The queue is sometimes unused when no logger is set.
    fn make_logger(
        #[allow(unused)] queue: crossbeam::channel::Receiver<LogEntry>,
        options: LoggingOptions,
    ) -> Result<Option<Box<dyn LogProcessor>>, ()> {
        Ok(match options {
            super::LoggingOptions::None => None,
            #[cfg(feature = "log-mongo")]
            super::LoggingOptions::Mongo(mongo_opts) => Some(Box::new(MongoLogger::new(
                mongodb::sync::Client::with_uri_str(mongo_opts.uri).map_err(|_| ())?,
                mongo_opts.db,
                mongo_opts.db_options,
                mongo_opts.collection,
                mongo_opts.col_options,
                queue,
            ))),
        })
    }
}

#[cfg(feature = "dot")]
mod inner {
    use std::collections::HashMap;
    use std::collections::HashSet;

    use graphviz_rust::dot_generator::*;
    use graphviz_rust::dot_structures::*;
    use rustc_hash::FxHashSet;

    use crate::datastructures::*;
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
