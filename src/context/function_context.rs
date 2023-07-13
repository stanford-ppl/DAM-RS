use dam_core::{identifier::Identifier, TimeManager};
use dam_macros::{cleanup, identifiable, time_managed};

use super::Context;

#[identifiable]
#[time_managed]
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

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {}
}
impl<RT> FunctionContext<RT>
where
    RT: FnOnce(&mut TimeManager) + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            run_fn: Default::default(),
            identifier: Identifier::new(),
            time: Default::default(),
        }
    }

    pub fn set_run(&mut self, run_fn: RT) {
        self.run_fn = Some(run_fn);
    }
}
