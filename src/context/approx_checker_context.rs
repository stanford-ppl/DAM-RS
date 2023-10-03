use dam_macros::context;

use super::Context;
use crate::{
    channel::{ChannelElement, DequeueResult, Receiver},
    types::DAMType,
};
use dam_core::prelude::*;

#[context]
pub struct ApproxCheckerContext<T: Clone, IType, FType, CheckType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
    CheckType: Send + Sync,
{
    iterator: Option<FType>,
    input: Receiver<T>,
    checker: CheckType,
}

impl<T: DAMType, IType, FType, CheckType> Context
    for ApproxCheckerContext<T, IType, FType, CheckType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
    CheckType: Fn(&T, &T) -> bool + Send + Sync,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        if let Some(iter) = self.iterator.take() {
            for (ind, val) in iter().enumerate() {
                match self.input.dequeue(&self.time) {
                    DequeueResult::Something(ChannelElement { time, data })
                        if !(self.checker)(&val, &data) =>
                    {
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

impl<T: DAMType, IType, FType, CheckType> ApproxCheckerContext<T, IType, FType, CheckType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
    CheckType: Fn(&T, &T) -> bool + Send + Sync,
{
    pub fn new(iterator: FType, input: Receiver<T>, checker: CheckType) -> Self {
        let gc = Self {
            iterator: Some(iterator),
            input,
            checker,
            context_info: Default::default(),
        };
        gc.input.attach_receiver(&gc);
        gc
    }
}
