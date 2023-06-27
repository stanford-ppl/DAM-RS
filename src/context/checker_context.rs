use crate::{
    channel::{utils::dequeue, ChannelElement, Receiver},
    types::DAMType,
};

use super::{view::TimeManager, Context};

pub struct CheckerContext<T, IType>
where
    IType: Iterator<Item = T>,
{
    time: TimeManager,
    iterator: fn() -> IType,
    input: Receiver<T>,
}

impl<T: DAMType, IType> Context for CheckerContext<T, IType>
where
    IType: Iterator<Item = T>,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        for (ind, val) in (self.iterator)().enumerate() {
            match dequeue(&mut self.time, &mut self.input) {
                Ok(ChannelElement { time, data }) if data != val => {
                    panic!("Mismatch on iteration {ind} at time {time:?}: Expected {val:?} but found {data:?}")
                }
                Ok(_) => {}
                Err(_) => panic!("Ran out of things to read on iteration {ind}, expected {val:?}"),
            }
        }
    }

    fn cleanup(&mut self) {
        self.time.cleanup();
    }

    fn view(&self) -> Box<dyn super::ContextView> {
        Box::new(self.time.view())
    }
}

impl<T: DAMType, IType> CheckerContext<T, IType>
where
    IType: Iterator<Item = T>,
{
    pub fn new(iterator: fn() -> IType, input: Receiver<T>) -> CheckerContext<T, IType> {
        let gc = CheckerContext {
            time: TimeManager::new(),
            iterator,
            input,
        };
        gc.input.attach_receiver(&gc);
        gc
    }
}
