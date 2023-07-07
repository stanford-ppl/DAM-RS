use crate::{
    channel::{utils::dequeue, ChannelElement, Receiver},
    types::DAMType,
};

use super::{
    view::{TimeManager, TimeView},
    Context,
};

pub struct CheckerContext<T, IType, FType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
{
    time: TimeManager,
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

    fn cleanup(&mut self) {
        self.time.cleanup();
    }

    fn view(&self) -> TimeView {
        self.time.view().into()
    }
}

impl<T: DAMType, IType, FType> CheckerContext<T, IType, FType>
where
    IType: Iterator<Item = T>,
    FType: FnOnce() -> IType + Send + Sync,
    CheckerContext<T, IType, FType>: Context,
{
    pub fn new(iterator: FType, input: Receiver<T>) -> CheckerContext<T, IType, FType> {
        let gc = CheckerContext {
            time: TimeManager::new(),
            iterator: Some(iterator),
            input,
        };
        gc.input.attach_receiver(&gc);
        gc
    }
}
