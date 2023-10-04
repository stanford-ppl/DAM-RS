use dam_core::prelude::*;
use dam_macros::context;

use crate::{
    channel::{ChannelElement, DequeueResult, Receiver},
    types::DAMType,
};

use crate::context::Context;

#[context]
pub struct CheckerContext<T: Clone, IType, FType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
{
    iterator: Option<FType>,
    input: Receiver<T>,
}

impl<T: DAMType, IType, FType> Context for CheckerContext<T, IType, FType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        if let Some(iter) = self.iterator.take() {
            for (ind, val) in iter().enumerate() {
                match self.input.dequeue(&self.time) {
                    DequeueResult::Something(ChannelElement { time, data }) if data != val => {
                        panic!("Mismatch on iteration {ind} at time {time:?}: Expected {val:?} but found {data:?}")
                    }
                    DequeueResult::Something(_) => {}
                    DequeueResult::Closed => {
                        panic!("Ran out of things to read on iteration {ind}, expected {val:?}")
                    }
                }
            }
        } else {
            panic!("Cannot run a Checker twice!");
        }
    }
}

impl<T: DAMType, IType, FType> CheckerContext<T, IType, FType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
{
    pub fn new(iterator: FType, input: Receiver<T>) -> CheckerContext<T, IType, FType> {
        let gc = CheckerContext {
            iterator: Some(iterator),
            input,
            context_info: Default::default(),
        };
        gc.input.attach_receiver(&gc);
        gc
    }
}
