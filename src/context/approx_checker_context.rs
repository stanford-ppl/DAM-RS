use dam_core::identifier::Identifier;
use dam_macros::{cleanup, identifiable, time_managed};

use crate::{
    channel::{utils::dequeue, ChannelElement, Receiver},
    types::DAMType,
};

use super::Context;

#[time_managed]
#[identifiable]
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
                match dequeue(&mut self.time, &mut self.input) {
                    Ok(ChannelElement { time, data }) if !(self.checker)(&val, &data) => {
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

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {}
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
            time: Default::default(),
            identifier: Identifier::new(),
        };
        gc.input.attach_receiver(&gc);
        gc
    }
}
