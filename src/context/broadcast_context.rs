use crate::{
    channel::{
        utils::{dequeue, enqueue},
        Receiver, Sender,
    },
    types::{Cleanable, DAMType},
};

use super::{view::TimeManager, Context};

pub struct BroadcastContext<T> {
    receiver: Receiver<T>,
    targets: Vec<Sender<T>>,
    time: TimeManager,
}

impl<T: DAMType> Context for BroadcastContext<T> {
    fn init(&mut self) {} // No-op

    fn run(&mut self) {
        loop {
            // println!("Running Broadcast Loop");
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

    fn cleanup(&mut self) {
        self.receiver.cleanup();
        self.targets.iter_mut().for_each(|target| target.cleanup());
        self.time.cleanup();
    }

    fn view(&self) -> super::view::TimeView {
        self.time.view().into()
    }
}

impl<T: DAMType> BroadcastContext<T> {
    pub fn new(receiver: Receiver<T>) -> Self {
        let x = Self {
            receiver,
            targets: vec![],
            time: TimeManager::default(),
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
    use super::BroadcastContext;

    use crate::{
        channel::bounded,
        context::{
            checker_context::CheckerContext, generator_context::GeneratorContext,
            parent::BasicParentContext, Context, ContextView, ParentContext,
        },
    };

    #[test]
    fn test_broadcast() {
        let test_size = 32;
        let num_checkers = 8;
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
