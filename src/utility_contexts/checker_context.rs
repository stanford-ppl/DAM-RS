use dam_macros::context_internal;

use crate::{
    channel::{ChannelElement, Receiver},
    types::DAMType,
};

use crate::context::Context;

/// An exact validation context for checking a channel against an iterator.
#[context_internal]
pub struct CheckerContext<T: Clone, IType, FType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
{
    iterator: Option<FType>,
    input: Receiver<T>,
}

impl<T: DAMType + PartialEq, IType, FType> Context for CheckerContext<T, IType, FType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        if let Some(iter) = self.iterator.take() {
            for (ind, val) in iter().enumerate() {
                match self.input.dequeue(&self.time) {
                    Ok(ChannelElement { time, data }) if data != val => {
                        panic!("Mismatch on iteration {ind} at time {time:?}: Expected {val:?} but found {data:?}")
                    }
                    Ok(_) => {}
                    Err(_) => {
                        panic!("Ran out of things to read on iteration {ind}, expected {val:?}")
                    }
                }
            }
        } else {
            panic!("Cannot run a Checker twice!");
        }
    }
}

impl<T: DAMType + PartialEq, IType, FType> CheckerContext<T, IType, FType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
{
    /// Constructs a new (exact) Checker -- for approximate checking use [super::ApproxCheckerContext]
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
