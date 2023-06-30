

use super::{
    view::{ContextView, TimeManager},
    Context,
};

pub struct FunctionContext<RT>
where
    RT: FnOnce(&mut TimeManager) + Send + Sync,
{
    pub time: TimeManager,
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

    fn cleanup(&mut self) {
        self.time.cleanup();
    }

    fn view(&self) -> Box<dyn ContextView> {
        Box::new(self.time.view())
    }
}
impl<RT> FunctionContext<RT>
where
    RT: FnOnce(&mut TimeManager) + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            time: TimeManager::new(),
            run_fn: None,
        }
    }

    pub fn set_run(&mut self, run_fn: RT) {
        self.run_fn = Some(run_fn);
    }
}
