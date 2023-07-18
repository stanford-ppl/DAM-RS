use dam_core::identifier::Identifier;
use dam_macros::{cleanup, identifiable, time_managed};

use crate::{
    channel::{utils::dequeue, ChannelElement, Receiver},
    types::DAMType,
};

use super::Context;

#[time_managed]
#[identifiable]
pub struct CheckerContext<T: Clone, IType, FType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
{
    iterator: Option<FType>,
    input: Receiver<T>,
}

impl<T: DAMType + std::cmp::PartialEq, IType, FType> Context for CheckerContext<T, IType, FType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        if let Some(iter) = self.iterator.take() {
            for (ind, val) in iter().enumerate() {
                match dequeue(&mut self.time, &mut self.input) {
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

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {}
}

impl<T: DAMType, IType, FType> CheckerContext<T, IType, FType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
    CheckerContext<T, IType, FType>: Context,
{
    pub fn new(iterator: FType, input: Receiver<T>) -> CheckerContext<T, IType, FType> {
        let gc = CheckerContext {
            iterator: Some(iterator),
            input,
            time: Default::default(),
            identifier: Identifier::new(),
        };
        gc.input.attach_receiver(&gc);
        gc
    }
}
