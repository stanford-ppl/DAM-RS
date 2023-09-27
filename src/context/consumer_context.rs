use dam_macros::{identifiable, time_managed};

use crate::{
    channel::Receiver,
    types::{Cleanable, DAMType},
};

use super::Context;

#[identifiable]
#[time_managed]
pub struct ConsumerContext<T: DAMType> {
    chan: Receiver<T>,
}

impl<T: DAMType> Context for ConsumerContext<T> {
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            if let crate::channel::DequeueResult::Closed = self.chan.dequeue(&mut self.time) {
                return;
            }
            self.time.incr_cycles(1);
        }
    }

    fn cleanup(&mut self) {
        self.chan.cleanup();
        self.time.cleanup();
    }
}

impl<T: DAMType> ConsumerContext<T> {
    pub fn new(chan: Receiver<T>) -> Self {
        let s = Self {
            chan,
            identifier: Default::default(),
            time: Default::default(),
        };
        s.chan.attach_receiver(&s);
        s
    }
}

#[identifiable]
#[time_managed]
pub struct PrinterContext<T: DAMType> {
    chan: Receiver<T>,
}

impl<T: DAMType> Context for PrinterContext<T> {
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            match self.chan.dequeue(&mut self.time) {
                crate::channel::DequeueResult::Something(x) => println!("{:?}", x),
                crate::channel::DequeueResult::Closed => return,
            }
            self.time.incr_cycles(1);
        }
    }

    fn cleanup(&mut self) {
        self.chan.cleanup();
        self.time.cleanup();
    }
}

impl<T: DAMType> PrinterContext<T> {
    pub fn new(chan: Receiver<T>) -> Self {
        let s = Self {
            chan,
            identifier: Default::default(),
            time: Default::default(),
        };
        s.chan.attach_receiver(&s);
        s
    }
}
