use crate::{
    channel::{utils::enqueue, ChannelElement, Sender},
    types::{Cleanable, DAMType},
};

use super::{view::TimeManager, Context};

pub struct GeneratorContext<T, IType, FType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
{
    time: TimeManager,
    iterator: Option<FType>,
    output: Sender<T>,
}

impl<T: DAMType, IType, FType> Context for GeneratorContext<T, IType, FType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        if let Some(func) = self.iterator.take() {
            for val in (func)() {
                let current_time = self.time.tick();
                enqueue(
                    &mut self.time,
                    &mut self.output,
                    ChannelElement::new(current_time, val),
                )
                .unwrap();
                self.time.incr_cycles(1);
            }
        } else {
            panic!("Can't run a generator twice!");
        }
    }

    fn cleanup(&mut self) {
        self.output.cleanup();
        self.time.cleanup();
    }

    fn view(&self) -> Box<dyn super::ContextView> {
        Box::new(self.time.view())
    }
}

impl<T: DAMType, IType, FType> GeneratorContext<T, IType, FType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
{
    pub fn new(iterator: FType, output: Sender<T>) -> GeneratorContext<T, IType, FType> {
        let gc = GeneratorContext {
            time: TimeManager::new(),
            iterator: Some(iterator),
            output,
        };
        gc.output.attach_sender(&gc);
        gc
    }
}
