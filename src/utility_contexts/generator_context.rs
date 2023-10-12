use dam_macros::context_internal;

use crate::context_tools::*;

use crate::context::Context;

/// A context which writes to a channel with elements taken from an iterator.
/// This is used for sending pre-defined values, or for reading from files.
#[context_internal]
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
    /// Constructs a GeneratorContext from an iterator and the output channel
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
