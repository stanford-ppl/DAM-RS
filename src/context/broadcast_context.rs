use dam_core::identifier::Identifier;
use dam_macros::{cleanup, identifiable, time_managed};

use crate::{
    channel::{
        utils::{dequeue, enqueue},
        Receiver, Sender,
    },
    types::{Cleanable, DAMType},
};

use super::Context;

#[time_managed]
#[identifiable]
pub struct BroadcastContext<T> {
    receiver: Receiver<T>,
    targets: Vec<Sender<T>>,
}

impl<T: DAMType> Context for BroadcastContext<T> {
    fn init(&mut self) {} // No-op

    fn run(&mut self) {
        loop {
            let value = dequeue(&mut self.time, &mut self.receiver);
            match value {
                Ok(mut data) => {
                    data.time = self.time.tick() + 1;
                    self.targets.iter_mut().for_each(|target| {
                        enqueue(&mut self.time, target, data.clone()).unwrap();
                    });
                    self.time.incr_cycles(1);
                }
                Err(_) => return,
            }
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.receiver.cleanup();
        self.targets.iter_mut().for_each(|target| target.cleanup());
    }
}

impl<T: DAMType> BroadcastContext<T> {
    pub fn new(receiver: Receiver<T>) -> Self {
        let x = Self {
            receiver,
            targets: vec![],

            identifier: Identifier::new(),
            time: Default::default(),
        };
        x.receiver.attach_receiver(&x);
        x
    }

    pub fn add_target(&mut self, target: Sender<T>) {
        target.attach_sender(self);
        self.targets.push(target);
    }
}

#[cfg(test)]
mod tests {
    use dam_core::{ContextView, TimeViewable};

    use super::BroadcastContext;

    use crate::{
        channel::bounded,
        context::{
            checker_context::CheckerContext, generator_context::GeneratorContext,
            parent::BasicParentContext, Context, ParentContext,
        },
    };

    #[test]
    fn test_broadcast() {
        let test_size = 32;
        let num_checkers = 256;
        let (init_send, init_recv) = bounded(8);

        let mut parent = BasicParentContext::default();
        let mut generator = GeneratorContext::new(move || (0..test_size), init_send);
        parent.add_child(&mut generator);

        let mut broadcast = BroadcastContext::new(init_recv);

        let mut checkers: Vec<_> = (0..num_checkers)
            .map(|_| {
                let (send, recv) = bounded(8);
                broadcast.add_target(send);
                let checker = CheckerContext::new(move || 0..test_size, recv);
                checker
            })
            .collect();

        checkers
            .iter_mut()
            .for_each(|checker| parent.add_child(checker));
        parent.add_child(&mut broadcast);

        parent.init();
        parent.run();
        parent.cleanup();
        dbg!(parent.view().tick_lower_bound());
    }
}
