use std::sync::Arc;

use dam_core::prelude::*;

use crate::{channel::handle::ChannelHandle, context::ContextSummary};

use super::{ProgramHelper, ProgramState};

pub struct Executed<'a> {
    pub(super) nodes: Vec<ContextSummary>,
    pub(super) edges: Vec<Arc<dyn ChannelHandle + 'a>>,
}

#[cfg(feature = "dot")]
use graphviz_rust::{dot_generator::*, dot_structures::*};

impl Executed<'_> {
    pub fn elapsed_cycles(&self) -> Option<Time> {
        self.nodes.iter().map(|node| node.max_time()).max()
    }

    #[cfg(feature = "dot")]
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

impl ProgramHelper for Executed<'_> {}

impl ProgramState for Executed<'_> {
    #[cfg(feature = "dot")]
    fn to_dot(&self) -> Graph {
        let mut stmts = vec![];

        for summary in &self.nodes {
            Self::add_node(summary, &mut stmts);
        }

        for edge in &self.edges {
            match (edge.sender(), edge.receiver()) {
                (None, _) => unreachable!("Should not have edges with no sender"),
                (Some(sender), None) => {
                    // Handle a void edge
                    let void_id = node_id!(edge.id());
                    stmts.push(Node::new(void_id.clone(), vec![attr!("label", esc "void")]).into());
                    stmts.push(
                        Edge {
                            ty: EdgeTy::Pair(
                                node_id!(Self::context_id_to_name(sender)).into(),
                                void_id.into(),
                            ),
                            attributes: vec![attr!("style", esc "dotted")],
                        }
                        .into(),
                    );
                }
                (Some(sender), Some(receiver)) => {
                    stmts.push(
                        Edge {
                            ty: EdgeTy::Pair(
                                node_id!(Self::context_id_to_name(sender)).into(),
                                node_id!(Self::context_id_to_name(receiver)).into(),
                            ),
                            attributes: vec![
                                attr!("label", esc edge.id()),
                                attr!("tooltip", esc format!("Capacity: {:?}\\nLatency: {}\\nRespLatency: {}", edge.spec().capacity(), edge.spec().latency(), edge.spec().resp_latency())),
                            ],
                        }
                        .into(),
                    );
                }
            }
        }

        Graph::DiGraph {
            id: Id::Plain("ProgramGraph".to_string()),
            strict: false,
            stmts,
        }
    }
}
