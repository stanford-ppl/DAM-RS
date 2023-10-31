//! Common utility contexts for testing, checking, etc.
//! These can be used to 'black-box' a lot of functionality, such as reading from a file for input, etc.

mod approx_checker_context;
mod broadcast_context;
mod checker_context;
mod consumer_context;
mod function_context;
mod generator_context;
mod trace_context;

pub use approx_checker_context::ApproxCheckerContext;
pub use broadcast_context::BroadcastContext;
pub use checker_context::CheckerContext;
pub use consumer_context::{ConsumerContext, PrinterContext};
pub use function_context::FunctionContext;
pub use generator_context::GeneratorContext;
pub use trace_context::{random_trace, TraceContext};
