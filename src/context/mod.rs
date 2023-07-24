use dam_core::{identifier::Identifiable, TimeViewable};

pub mod broadcast_context;
pub mod checker_context;
pub mod function_context;
pub mod generator_context;
pub mod parent;
pub mod print_context;

pub trait Context: Send + Sync + TimeViewable + Identifiable {
    fn init(&mut self);
    fn run(&mut self);
    fn cleanup(&mut self);
}
