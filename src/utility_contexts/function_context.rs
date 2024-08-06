use crate::{context_tools::*, view::TimeManager};
use dam_macros::context_internal;

use super::UtilityError;

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

    fn run_falliable(&mut self) -> anyhow::Result<()> {
        if let Some(rf) = self.run_fn.take() {
            rf(&mut self.time);
            Ok(())
        } else {
            Err(UtilityError::DuplicateExec)?
        }
    }
}
impl<RT> FunctionContext<RT>
where
    Self: Context,
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
    Self: Context,
{
    fn default() -> Self {
        Self::new()
    }
}
