use dam_macros::context_internal;

use crate::{
    channel::{Receiver, Sender},
    types::DAMType,
};

use crate::context::Context;

/// Since DAM channels are single-producer single-consumer, Broadcasts can be used to send from a single channel to multiple channels.
#[context_internal]
pub struct BroadcastContext<T: Clone> {
    receiver: Receiver<T>,
    targets: Vec<Sender<T>>,
}

impl<T: DAMType> Context for BroadcastContext<T> {
    fn run(&mut self) {
        loop {
            let value = self.receiver.dequeue(&self.time);
            match value {
                Ok(mut data) => {
                    data.time = self.time.tick() + 1;
                    self.targets.iter().for_each(|target| {
                        target.enqueue(&self.time, data.clone()).unwrap();
                    });
                    self.time.incr_cycles(1);
                }
                Err(_) => return,
            }
        }
    }
}

impl<T: DAMType> BroadcastContext<T> {
    /// Sets up a broadcast context with an empty target list.
    pub fn new(receiver: Receiver<T>) -> Self {
        let x = Self {
            receiver,
            targets: vec![],
            context_info: Default::default(),
        };
        x.receiver.attach_receiver(&x);
        x
    }

    /// Registers a target for the broadcast
    pub fn add_target(&mut self, target: Sender<T>) {
        target.attach_sender(self);
        self.targets.push(target);
    }
}

#[cfg(test)]
mod tests {

    use super::BroadcastContext;

    use crate::{
        simulation::{InitializationOptions, ProgramBuilder, RunOptions},
        utility_contexts::{CheckerContext, GeneratorContext},
    };

    #[test]
    fn test_broadcast() {
        let test_size = 32;
        let num_checkers = 256;
        let mut parent = ProgramBuilder::default();
        let (init_send, init_recv) = parent.bounded(8);

        let generator = GeneratorContext::new(move || (0..test_size), init_send);
        parent.add_child(generator);

        let mut broadcast = BroadcastContext::new(init_recv);

        let checkers: Vec<_> = (0..num_checkers)
            .map(|_| {
                let (send, recv) = parent.bounded(8);
                broadcast.add_target(send);

                CheckerContext::new(move || 0..test_size, recv)
            })
            .collect();

        checkers
            .into_iter()
            .for_each(|checker| parent.add_child(checker));
        parent.add_child(broadcast);

        parent
            .initialize(InitializationOptions::default())
            .unwrap()
            .run(RunOptions::default());
    }
}
