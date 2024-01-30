use crate::{
    datastructures::Time,
    logging::{initialize_log, LogEntry, LogInterface, LogProcessor},
    shim::spawn,
};

#[cfg(feature = "log-mongo")]
use crate::logging::mongo_logger::{mongodb, MongoLogger};

use super::{executed::Executed, programdata::ProgramData, LoggingOptions, RunOptions};

/// An initialized program, which has passed checking after the [super::ProgramBuilder]
pub struct Initialized<'a> {
    pub(super) data: ProgramData<'a>,
}

const NUM_LOGGERS: usize = 4;

impl<'a> Initialized<'a> {
    /// Executes the program with specified options.
    /// Currently will deadlock frequently if there is an error at runtime, due to blocking dequeues.
    pub fn run(mut self, options: RunOptions) -> Executed<'a> {
        // If we should make a log, then we populate this stuff

        // This guard is necessary because when logging is off, then the LoggingOptions enum is always None.
        #[allow(irrefutable_let_patterns)]
        let (log_sender, log_receiver, has_logger) = if let LoggingOptions::None = options.logging {
            // don't log
            (None, None, false)
        } else {
            // Limit logger size to at most some number of elements at a time to prevent an infinitely growing log.
            // Sinze the batch size for mongo is 100k, we'll be generous and allow 16 batches in the channel at a time.
            let (log_sender, log_receiver) = crossbeam::channel::bounded(100000 * 16);
            (Some(log_sender), Some(log_receiver), true)
        };

        let summaries = std::sync::Arc::new(crossbeam::queue::SegQueue::new());

        crate::shim::scope(|s| {
            let base_time = std::time::Instant::now();

            self.data.nodes.drain(..).for_each(|mut child| {
                let id = child.id();
                let name = child.name();
                let builder = crate::shim::make_builder(options.mode).name(format!(
                    "{}({})",
                    child.id(),
                    child.name()
                ));
                let filter_copy = options.log_filter.clone();

                let sender = log_sender.clone();
                let summary_handle = summaries.clone();

                spawn!(s, builder, move || {
                    if has_logger {
                        let active_filter = match filter_copy {
                            super::LogFilterKind::Blanket(filter) => filter,
                            super::LogFilterKind::PerChild(func) => func(child.id()),
                        };
                        if let Some(snd) = sender {
                            initialize_log(LogInterface::new(
                                child.id(),
                                snd,
                                base_time,
                                active_filter,
                                Time::new(0),
                            ));
                        }
                    }
                    child.run();
                    summary_handle.push(child.summarize());
                })
                .unwrap_or_else(|_| panic!("Failed to spawn child {name:?} {id:?}"));
            });

            drop(log_sender);

            if let Some(receiver) = log_receiver {
                for _ in 0..NUM_LOGGERS {
                    let builder = crate::shim::make_builder(options.mode);
                    Self::make_logger(receiver.clone(), options.logging.clone())
                        .expect("Error creating logger!")
                        .map(|mut exec_logger| spawn!(s, builder, move || exec_logger.spawn()));
                }
            }
        });

        Executed {
            nodes: std::sync::Arc::into_inner(summaries)
                .expect("Could not obtain unique access to summaries")
                .into_iter()
                .collect(),
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
            let node_graph: HashMap<_, _> =
                self.data.nodes.iter().flat_map(|node| node.ids()).collect();

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
                .flat_map(|edge| Self::generate_edge(edge.clone()))
                .collect()
        }
    }
}
