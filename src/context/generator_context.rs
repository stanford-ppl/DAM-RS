use dam_core::identifier::Identifier;
use dam_macros::{cleanup, identifiable, time_managed};

use crate::{
    channel::{utils::enqueue, ChannelElement, Sender},
    types::{Cleanable, DAMType},
};

use super::Context;

#[identifiable]
#[time_managed]
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

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.output.cleanup();
        let curr_time = self.time.tick();
        println!("Generator");
        dbg!(curr_time);
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
            identifier: Identifier::new(),
            time: Default::default(),
        };
        gc.output.attach_sender(&gc);
        gc
    }
}
