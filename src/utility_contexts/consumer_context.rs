use dam_macros::context;

use crate::{channel::Receiver, types::DAMType};
use dam_core::prelude::*;

use crate::context::Context;

#[context]
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
                Ok(x) => println!("{:?}", x),
                Err(_) => return,
            }
            self.time.incr_cycles(1);
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
