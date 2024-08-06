use std::sync::Arc;

use crate::{channel::handle::ChannelHandle, context::ContextSummary};

use super::SimulationError;

/// Represents a program graph which has been executed.
/// This still stores all of the edges in the graph, but each node is replaced with its summary.
pub struct Executed<'a> {
    pub(super) nodes: Vec<ContextSummary>,
    pub(super) failures: Vec<SimulationError>,

    // Edges might not be used if the dot cfg isn't enabled.
    #[allow(unused)]
    pub(super) edges: Vec<Arc<dyn ChannelHandle + 'a>>,
}

impl Executed<'_> {
    /// Gets the total number of cycles taken by the graph, defined as the maximum number of cycles taken by any node.
    /// This is a slight underapproximation in the event that the last nodes scheduled their output for a time in the future.
    pub fn elapsed_cycles(&self) -> Option<u64> {
        self.nodes.iter().map(|node| node.max_time()).max()
    }

    /// Returns if simulation was successful with no errors.
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }

    /// Executes a given function on program failures.
    pub fn run_failures<R>(&self, f: impl FnOnce(&Vec<SimulationError>) -> R) -> R {
        f(&self.failures)
    }

    /// Prints all of the failures in the program
    pub fn dump_failures(&self) {
        println!("{:?}", self.failures);
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "dot")] {
        use super::dot::DotConvertibleHelper;
        use crate::view::ContextView;
        use graphviz_rust::{dot_generator::*, dot_structures::*};

        impl Executed<'_> {
            fn add_node(summary: &ContextSummary, stmts: &mut Vec<Stmt>) {
                let label_string = format!("{}({})", summary.id.name, summary.id.id);
                if summary.children.is_empty() {
                    stmts.push(
                        Node::new(
                            node_id!(Self::context_id_to_name(summary.id.id)),
                            vec![
                                attr!("shape", esc "rectangle"),
                                attr!("label", esc label_string),
                                attr!("tooltip", esc format!("Elapsed: {}", summary.time.tick_lower_bound().time())),
                            ],
                        )
                        .into(),
                    );
                } else {
                    let mut inner_stmts = vec![
                        stmt!(attr!("label", esc label_string)),
                        stmt!(
                            attr!("tooltip", esc format!("Elapsed: {}", summary.time.tick_lower_bound().time()))
                        ),
                    ];

                    for child in &summary.children {
                        Self::add_node(child, &mut inner_stmts);
                    }

                    stmts.push(
                        Subgraph {
                            id: Id::Plain(format!(
                                "cluster_{}",
                                Self::context_id_to_name(summary.id.id)
                            )),
                            stmts: inner_stmts,
                        }
                        .into(),
                    );
                }
            }
        }

        impl DotConvertibleHelper for Executed<'_> {
            fn add_nodes(&self) -> Vec<Stmt> {
                let mut stmts = vec![];
                for summary in &self.nodes {
                    Self::add_node(summary, &mut stmts);
                }
                stmts
            }

            fn generate_edges(&self) -> Vec<Stmt> {
                self.edges
                    .iter()
                    .flat_map(|edge| Self::generate_edge(edge.clone()))
                    .collect()
            }
        }

    }
}
