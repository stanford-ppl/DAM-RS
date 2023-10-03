use dam_macros::context;

use crate::{
    channel::Receiver,
    types::{Cleanable, DAMType},
};
use dam_core::prelude::*;

use super::Context;

#[context]
pub struct ConsumerContext<T: DAMType> {
    chan: Receiver<T>,
}

impl<T: DAMType> Context for ConsumerContext<T> {
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            if let crate::channel::DequeueResult::Closed = self.chan.dequeue(&self.time) {
                return;
            }
            self.time.incr_cycles(1);
        }
    }
}

impl<T: DAMType> ConsumerContext<T> {
    pub fn new(chan: Receiver<T>) -> Self {
        let s = Self {
            chan,
            context_info: Default::default(),
        };
        s.chan.attach_receiver(&s);
        s
    }
}

#[context]
pub struct PrinterContext<T: DAMType> {
    chan: Receiver<T>,
}

impl<T: DAMType> Context for PrinterContext<T> {
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            match self.chan.dequeue(&self.time) {
                crate::channel::DequeueResult::Something(x) => println!("{:?}", x),
                crate::channel::DequeueResult::Closed => return,
            }
            &mut &mut self.time.incr_cycles(1);
        }
    }
}

impl<T: DAMType> PrinterContext<T> {
    pub fn new(chan: Receiver<T>) -> Self {
        let s = Self {
            chan,
            context_info: Default::default(),
        };
        s.chan.attach_receiver(&s);
        s
    }
}
