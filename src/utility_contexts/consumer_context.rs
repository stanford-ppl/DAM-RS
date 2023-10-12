use dam_macros::context_internal;

use crate::{channel::Receiver, types::DAMType};

use crate::context::Context;

/// A context which simply consumes values out of a channel
/// This is useful for when it is not feasible to construct a Void sender instead via [crate::simulation::ProgramBuilder::void] instead
#[context_internal]
pub struct ConsumerContext<T: DAMType> {
    chan: Receiver<T>,
}

impl<T: DAMType> Context for ConsumerContext<T> {
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            if self.chan.dequeue(&self.time).is_err() {
                return;
            }
            self.time.incr_cycles(1);
        }
    }
}

impl<T: DAMType> ConsumerContext<T> {
    /// Constructs a context which reads out of a channel
    pub fn new(chan: Receiver<T>) -> Self {
        let s = Self {
            chan,
            context_info: Default::default(),
        };
        s.chan.attach_receiver(&s);
        s
    }
}

/// A context which prints elements of a channel to STDOUT
/// Basically for debugging only, as it can emit a large amout of text.
#[context_internal]
pub struct PrinterContext<T: DAMType> {
    chan: Receiver<T>,
}

impl<T: DAMType> Context for PrinterContext<T> {
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            match self.chan.dequeue(&self.time) {
                Ok(x) => println!("{:?}", x),
                Err(_) => return,
            }
            self.time.incr_cycles(1);
        }
    }
}

impl<T: DAMType> PrinterContext<T> {
    /// Constructs a PrinterContext from a receiver.
    pub fn new(chan: Receiver<T>) -> Self {
        let s = Self {
            chan,
            context_info: Default::default(),
        };
        s.chan.attach_receiver(&s);
        s
    }
}
