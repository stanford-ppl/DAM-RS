use dam_core::prelude::*;
use dam_macros::context;

use crate::{
    channel::{ChannelElement, Sender},
    types::DAMType,
};

use crate::context::Context;

#[context]
pub struct GeneratorContext<T: Clone, IType, FType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
{
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
                let current_time: Time = self.time.tick();
                self.output
                    .enqueue(&self.time, ChannelElement::new(current_time + 1, val))
                    .unwrap();
                self.time.incr_cycles(1);
            }
        } else {
            panic!("Can't run a generator twice!");
        }
    }
}

impl<T: DAMType, IType, FType> GeneratorContext<T, IType, FType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
{
    pub fn new(iterator: FType, output: Sender<T>) -> GeneratorContext<T, IType, FType> {
        let gc = GeneratorContext {
            iterator: Some(iterator),
            output,
            context_info: Default::default(),
        };
        gc.output.attach_sender(&gc);
        gc
    }
}
