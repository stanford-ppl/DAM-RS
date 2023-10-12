use crate::{context_tools::*, view::TimeManager};
use dam_macros::context_internal;

/// Contains an arbitrarily defined inner body for a context
/// Used mostly for one-off operations, such as test drivers.
#[context_internal]
pub struct FunctionContext<RT> {
    run_fn: Option<RT>,
}

impl<RT> Context for FunctionContext<RT>
where
    RT: FnOnce(&mut TimeManager) + Send + Sync,
{
    fn init(&mut self) {} //No-op since Function Contexts don't have internal data.

    fn run(&mut self) {
        if let Some(rf) = self.run_fn.take() {
            rf(&mut self.time);
        } else {
            panic!("Called run twice!");
        }
    }
}
impl<RT> FunctionContext<RT>
where
    RT: FnOnce(&mut TimeManager) + Send + Sync,
{
    /// Constructs an empty FunctionContext
    pub fn new() -> Self {
        Self {
            run_fn: Default::default(),
            context_info: Default::default(),
        }
    }

    /// Sets the run function for the context.
    pub fn set_run(&mut self, run_fn: RT) {
        self.run_fn = Some(run_fn);
    }
}

impl<RT> Default for FunctionContext<RT>
where
    RT: FnOnce(&mut TimeManager) + Send + Sync,
{
    fn default() -> Self {
        Self::new()
    }
}
