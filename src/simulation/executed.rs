use std::sync::Arc;

use crate::{
    channel::handle::ChannelHandle, context::ContextSummary, datastructures::Time,
    view::ContextView,
};

/// Represents a program graph which has been executed.
/// This still stores all of the edges in the graph, but each node is replaced with its summary.
pub struct Executed<'a> {
    pub(super) nodes: Vec<ContextSummary>,

    // Edges might not be used if the dot cfg isn't enabled.
    #[allow(unused)]
    pub(super) edges: Vec<Arc<dyn ChannelHandle + 'a>>,
}

#[cfg(feature = "dot")]
use graphviz_rust::{dot_generator::*, dot_structures::*};

#[cfg(feature = "dot")]
use super::dot::DotConvertibleHelper;

impl Executed<'_> {
    /// Gets the total number of cycles taken by the graph, defined as the maximum number of cycles taken by any node.
    /// This is a slight underapproximation in the event that the last nodes scheduled their output for a time in the future.
    pub fn elapsed_cycles(&self) -> Option<Time> {
        self.nodes.iter().map(|node| node.max_time()).max()
    }
}

#[cfg(feature = "dot")]
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

#[cfg(feature = "dot")]
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
            .map(|edge| Self::generate_edge(edge.clone()))
            .flatten()
            .collect()
    }
}
