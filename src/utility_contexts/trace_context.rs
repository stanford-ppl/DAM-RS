use dam_macros::context_internal;

use crate::context_tools::*;

use crate::context::Context;
use crate::datastructures::{SyncSendMarker, Time};

/// A context which writes to a channel with elements and timings taken from an iterator.
/// If only elements are desired, use a [super::GeneratorContext].
/// This is used for sending pre-defined values, or for reading from files.
#[context_internal]
pub struct TraceContext<T: Clone, IType, FType, TimeType> {
    iterator: Option<FType>,
    output: Sender<T>,

    _marker: SyncSendMarker<(IType, TimeType)>,
}

impl<T: DAMType, IType, FType, TimeType> Context for TraceContext<T, IType, FType, TimeType>
where
    IType: Iterator<Item = (T, TimeType)>,
    FType: FnOnce() -> IType + Send + Sync,
    TimeType: Into<Time>,
{
    fn run(&mut self) {
        if let Some(func) = self.iterator.take() {
            for (data, time) in (func)() {
                self.time.advance(time.into() - 1);
                self.output
                    .enqueue(&self.time, ChannelElement::new(self.time.tick() + 1, data))
                    .unwrap();
                // Don't need an incr_cycles in case the generator wants to emit multiple values on the same cycle.
            }
        } else {
            panic!("Can't run a trace twice!");
        }
    }
}

impl<T: DAMType, IType, FType, TimeType> TraceContext<T, IType, FType, TimeType>
where
    Self: Context,
{
    /// Constructs a [TraceContext] from an iterator and the output channel
    pub fn new(iterator: FType, output: Sender<T>) -> Self {
        let gc = TraceContext {
            iterator: Some(iterator),
            output,
            context_info: Default::default(),
            _marker: Default::default(),
        };
        gc.output.attach_sender(&gc);
        gc
    }
}

/// A function to generate a random trace w/ monotonically non-decreasing timestamps.
pub fn random_trace(
    length: usize,
    min_step: u64,
    max_step: u64,
) -> impl Iterator<Item = (usize, Time)> {
    let mut trng = fastrand::Rng::new();
    let mut cur_time = Time::new(0);
    (0..length).map(move |i| {
        cur_time += trng.u64(min_step..=max_step);
        (i, cur_time)
    })
}
