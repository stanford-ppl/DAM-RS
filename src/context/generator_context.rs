use crate::{
    channel::{utils::enqueue, ChannelElement, Sender},
    types::{Cleanable, DAMType},
};

use super::{view::TimeManager, Context};

pub struct GeneratorContext<T, IType>
where
    IType: Iterator<Item = T>,
{
    time: TimeManager,
    iterator: fn() -> IType,
    output: Sender<T>,
}

impl<T: DAMType, IType> Context for GeneratorContext<T, IType>
where
    IType: Iterator<Item = T>,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        for val in (self.iterator)() {
            let current_time = self.time.tick();
            enqueue(
                &mut self.time,
                &mut self.output,
                ChannelElement::new(current_time, val),
            )
            .unwrap();
            self.time.incr_cycles(1);
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

impl<T: DAMType, IType> GeneratorContext<T, IType>
where
    IType: Iterator<Item = T>,
{
    pub fn new(iterator: fn() -> IType, output: Sender<T>) -> GeneratorContext<T, IType> {
        let gc = GeneratorContext {
            time: TimeManager::new(),
            iterator,
            output,
        };
        gc.output.attach_sender(&gc);
        gc
    }
}
