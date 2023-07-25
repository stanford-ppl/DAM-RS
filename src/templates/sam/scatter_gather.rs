use dam_core::identifier::Identifier;
use dam_macros::{cleanup, identifiable, time_managed};

use crate::{
    channel::{
        utils::{dequeue, enqueue},
        ChannelElement, Receiver, Sender,
    },
    context::Context,
    types::{Cleanable, DAMType},
};

use super::primitive::Token;

#[time_managed]
#[identifiable]
pub struct Scatter<ValType: Clone, StopType: Clone> {
    receiver: Receiver<Token<ValType, StopType>>,
    targets: Vec<Sender<Token<ValType, StopType>>>,
}

impl<ValType, StopType> Context for Scatter<ValType, StopType>
where
    ValType: DAMType
        + std::ops::Mul<ValType, Output = ValType>
        + std::ops::Add<ValType, Output = ValType>
        + std::cmp::PartialOrd<ValType>,
    StopType: DAMType + std::ops::Add<u32, Output = StopType>,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        let mut target_idx = 0;
        loop {
            match dequeue(&mut self.time, &mut self.receiver) {
                Ok(mut curr_in) => {
                    match curr_in.data {
                        Token::Val(_) => {
                            curr_in.time = self.time.tick() + 1;
                            enqueue(&mut self.time, &mut self.targets[target_idx], curr_in)
                                .unwrap();
                            // Round robin send
                            target_idx = (target_idx + 1) % self.targets.len();
                        }
                        tkn @ Token::Stop(_) | tkn @ Token::Done => {
                            let channel_elem = ChannelElement::new(self.time.tick() + 1, tkn);
                            self.targets.iter_mut().for_each(|target| {
                                enqueue(&mut self.time, target, channel_elem.clone()).unwrap();
                            });
                        }
                        _ => {
                            panic!("Undefined case found in scatter");
                        }
                    };
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

impl<ValType: DAMType, StopType: DAMType> Scatter<ValType, StopType>
where
    Scatter<ValType, StopType>: Context,
{
    pub fn new(receiver: Receiver<Token<ValType, StopType>>) -> Self {
        let x = Self {
            receiver,
            targets: vec![],
            identifier: Identifier::new(),
            time: Default::default(),
        };
        x.receiver.attach_receiver(&x);
        x
    }

    pub fn add_target(&mut self, target: Sender<Token<ValType, StopType>>) {
        target.attach_sender(self);
        self.targets.push(target);
    }
}

#[time_managed]
#[identifiable]
pub struct Gather<ValType: Clone, StopType: Clone> {
    targets: Vec<Receiver<Token<ValType, StopType>>>,
    merged: Sender<Token<ValType, StopType>>,
}

impl<ValType, StopType> Context for Gather<ValType, StopType>
where
    ValType: DAMType
        + std::ops::Mul<ValType, Output = ValType>
        + std::ops::Add<ValType, Output = ValType>
        + std::cmp::PartialOrd<ValType>,
    StopType: DAMType + std::ops::Add<u32, Output = StopType> + std::cmp::PartialEq,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        let mut target_idx = 0;
        loop {
            let output = dequeue(&mut self.time, &mut self.targets[target_idx])
                .unwrap()
                .data;

            let channel_elem = ChannelElement::new(self.time.tick() + 1, output.clone());
            enqueue(&mut self.time, &mut self.merged, channel_elem).unwrap();

            match output.clone() {
                tkn @ Token::Stop(_) | tkn @ Token::Done => {
                    for (idx, chan) in self.targets.iter_mut().enumerate() {
                        if idx == target_idx {
                            continue;
                        }
                        dequeue(&mut self.time, chan).unwrap();
                    }
                    if tkn == Token::Done {
                        return;
                    }
                }
                Token::Val(_) => {
                    target_idx = (target_idx + 1) % self.targets.len();
                }
                _ => todo!(),
            }
            self.time.incr_cycles(1);
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.merged.cleanup();
        self.targets.iter_mut().for_each(|target| target.cleanup());
    }
}

impl<ValType: DAMType, StopType: DAMType> Gather<ValType, StopType>
where
    Gather<ValType, StopType>: Context,
{
    pub fn new(merged: Sender<Token<ValType, StopType>>) -> Self {
        let x = Self {
            merged,
            targets: vec![],
            identifier: Identifier::new(),
            time: Default::default(),
        };
        x.merged.attach_sender(&x);
        x
    }

    pub fn add_target(&mut self, target: Receiver<Token<ValType, StopType>>) {
        target.attach_receiver(self);
        self.targets.push(target);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        context::{
            checker_context::CheckerContext, generator_context::GeneratorContext, Context,
        },
        simulation::Program,
        templates::sam::primitive::Token,
        token_vec,
    };

    use super::{Gather, Scatter};

    #[test]
    fn scatter_2d_test() {
        let in_ref2 = || token_vec!(u32; u32; 0, 1, 2, "S0", 0, 1, 2, "S1", "D").into_iter();

        let out_crd2 = || token_vec!(u32; u32; 0, 2, "S0", 1, "S1", "D").into_iter();
        let out_ref2 = || token_vec!(u32; u32; 1, "S0", 0, 2, "S1", "D").into_iter();
        scatter_test(in_ref2, out_crd2, out_ref2);
    }

    #[test]
    fn scatter_2d_test1() {
        let in_ref2 = || token_vec!(u32; u32; 0, "S0", 0, 1, 2, "S1", "D").into_iter();

        let out_crd2 = || token_vec!(u32; u32; 0, "S0", 1, "S1", "D").into_iter();
        let out_ref2 = || token_vec!(u32; u32; "S0", 0, 2, "S1", "D").into_iter();
        scatter_test(in_ref2, out_crd2, out_ref2);
    }

    #[test]
    fn gather_2d_test() {
        let out_ref2 =
            || token_vec!(u32; u32; 0, 1, 2, 3, "S0", 0, 1, 2, 3, 4, "S1", "D").into_iter();

        let in_crd2 = || token_vec!(u32; u32; 0, 2, "S0", 0, 2, 4, "S1", "D").into_iter();
        let in_ref2 = || token_vec!(u32; u32; 1, 3, "S0", 1, 3, "S1", "D").into_iter();
        gather_test(in_crd2, in_ref2, out_ref2);
    }

    #[test]
    fn gather_2d_test1() {
        let out_ref2 = || token_vec!(u32; u32; 0, "S0", 0, 1, 2, "S1", "D").into_iter();

        let in_crd2 = || token_vec!(u32; u32; 0, "S0", 1, "S1", "D").into_iter();
        let in_ref2 = || token_vec!(u32; u32; "S0", 0, 2, "S1", "D").into_iter();
        gather_test(in_crd2, in_ref2, out_ref2);
    }

    fn scatter_test<IRT2, ORT1, ORT2>(
        in_ref2: fn() -> IRT2,
        out_crd2: fn() -> ORT1,
        out_ref2: fn() -> ORT2,
    ) where
        IRT2: Iterator<Item = Token<u32, u32>> + 'static,
        ORT1: Iterator<Item = Token<u32, u32>> + 'static,
        ORT2: Iterator<Item = Token<u32, u32>> + 'static,
    {
        let mut parent = Program::default();
        let chan_size = 128;

        let (in_ref2_sender, in_ref2_receiver) = parent.bounded::<Token<u32, u32>>(chan_size);
        let (out_crd_sender, out_crd_receiver) = parent.bounded::<Token<u32, u32>>(chan_size);
        let (out_ref2_sender, out_ref2_receiver) = parent.bounded::<Token<u32, u32>>(chan_size);

        let mut scat = Scatter::new(in_ref2_receiver);
        scat.add_target(out_crd_sender);
        scat.add_target(out_ref2_sender);
        let gen4 = GeneratorContext::new(in_ref2, in_ref2_sender);
        let crd_checker = CheckerContext::new(out_crd2, out_crd_receiver);
        let ref2_checker = CheckerContext::new(out_ref2, out_ref2_receiver);
        parent.add_child(gen4);
        parent.add_child(crd_checker);
        parent.add_child(ref2_checker);
        parent.add_child(scat);
        parent.init();
        parent.run();
    }

    fn gather_test<IRT2, ORT1, ORT2>(
        in_crd2: fn() -> ORT1,
        in_ref2: fn() -> ORT2,
        out_ref2: fn() -> IRT2,
    ) where
        IRT2: Iterator<Item = Token<u32, u32>> + 'static,
        ORT1: Iterator<Item = Token<u32, u32>> + 'static,
        ORT2: Iterator<Item = Token<u32, u32>> + 'static,
    {
        let mut parent = Program::default();
        let chan_size = 128;

        let (out_ref2_sender, out_ref2_receiver) = parent.bounded::<Token<u32, u32>>(chan_size);
        let (in_crd_sender, in_crd_receiver) = parent.bounded::<Token<u32, u32>>(chan_size);
        let (in_ref2_sender, in_ref2_receiver) = parent.bounded::<Token<u32, u32>>(chan_size);

        let mut gat = Gather::new(out_ref2_sender);
        gat.add_target(in_crd_receiver);
        gat.add_target(in_ref2_receiver);

        let gen3 = GeneratorContext::new(in_crd2, in_crd_sender);
        let gen4 = GeneratorContext::new(in_ref2, in_ref2_sender);
        let crd_checker = CheckerContext::new(out_ref2, out_ref2_receiver);
        parent.add_child(gen3);
        parent.add_child(gen4);
        parent.add_child(crd_checker);
        parent.add_child(gat);
        parent.init();
        parent.run();
    }
}
