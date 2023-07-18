use dam_core::{identifier::Identifiable, log_graph::get_graph, TimeViewable};

pub mod broadcast_context;
pub mod checker_context;
pub mod function_context;
pub mod generator_context;
pub mod parent;

pub trait Context: Send + Sync + TimeViewable + Identifiable {
    fn init(&mut self);
    fn run(&mut self);
    fn cleanup(&mut self);

    fn register(&self) {
        get_graph().register(self.id(), self.name());
    }
}
