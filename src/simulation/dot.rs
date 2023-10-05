use std::sync::Arc;

use dam_core::prelude::Identifier;
use graphviz_rust::{
    dot_generator::*,
    dot_structures::*,
    printer::{DotPrinter, PrinterContext},
};

use crate::channel::handle::ChannelHandle;

pub trait DotConvertible {
    fn to_dot(&self) -> Graph;

    fn to_dot_string(&self) -> String {
        self.to_dot().print(&mut PrinterContext::default())
    }
}

pub(super) trait DotConvertibleHelper {
    fn context_id_to_name(id: Identifier) -> String {
        format!("Node_{}", id.id)
    }

    fn add_nodes(&self) -> Vec<Stmt>;
    fn generate_edges(&self) -> Vec<Stmt>;

    fn generate_edge<'a>(edge: Arc<dyn ChannelHandle + 'a>) -> Vec<Stmt> {
        let mut stmts = vec![];
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
        stmts
    }
}

impl<T: DotConvertibleHelper> DotConvertible for T {
    fn to_dot(&self) -> Graph {
        let mut stmts = self.add_nodes();

        stmts.extend(self.generate_edges());

        Graph::DiGraph {
            id: Id::Plain("ProgramGraph".to_string()),
            strict: false,
            stmts,
        }
    }
}
