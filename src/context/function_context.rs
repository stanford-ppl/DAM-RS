use super::{
    view::{ContextView, TimeManager},
    Context,
};
use std::sync::Arc;

type FType = dyn Fn(&mut FunctionContext) + Send + Sync;
pub struct FunctionContext {
    pub time: TimeManager,
    init_fn: Arc<FType>,
    run_fn: Arc<FType>,
    cleanup_fn: Arc<FType>,
}

impl Context for FunctionContext {
    fn init(&mut self) {
        (self.init_fn.clone())(self);
    }

    fn run(&mut self) {
        (self.run_fn.clone())(self);
    }

    fn cleanup(&mut self) {
        (self.cleanup_fn.clone())(self);
        self.time.cleanup();
    }

    fn view(&self) -> Box<dyn ContextView> {
        Box::new(self.time.view())
    }
}
impl FunctionContext {
    fn do_nothing() -> Arc<FType> {
        Arc::new(|_| {})
    }

    pub fn set_init(&mut self, init: Arc<FType>) {
        self.init_fn = init;
    }

    pub fn set_run(&mut self, run: Arc<FType>) {
        self.run_fn = run;
    }

    pub fn set_cleanup(&mut self, cleanup: Arc<FType>) {
        self.cleanup_fn = cleanup;
    }
}

impl Default for FunctionContext {
    fn default() -> FunctionContext {
        FunctionContext {
            time: TimeManager::new(),
            init_fn: FunctionContext::do_nothing(),
            run_fn: FunctionContext::do_nothing(),
            cleanup_fn: FunctionContext::do_nothing(),
        }
    }
}
