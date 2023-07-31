use dam_core::identifier::Identifier;
use dam_core::TimeManager;
use dam_macros::{cleanup, identifiable, time_managed};

use crate::{
    channel::{
        self,
        utils::{dequeue, enqueue, Peekable},
        ChannelElement, Receiver, Sender,
    },
    context::Context,
    types::{Cleanable, DAMType},
};

use super::primitive::Token;

#[time_managed]
#[identifiable]
pub struct StknDrop<ValType: Clone, StopType: Clone> {
    pub in_val: Receiver<Token<ValType, StopType>>,
    pub out_val: Sender<Token<ValType, StopType>>,
}

impl<ValType: DAMType, StopType: DAMType> StknDrop<ValType, StopType>
where
    StknDrop<ValType, StopType>: Context,
{
    pub fn new(
        in_val: Receiver<Token<ValType, StopType>>,
        out_val: Sender<Token<ValType, StopType>>,
    ) -> Self {
        let stkn_drop = StknDrop {
            in_val,
            out_val,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (stkn_drop).in_val.attach_receiver(&stkn_drop);
        (stkn_drop).out_val.attach_sender(&stkn_drop);

        stkn_drop
    }
}

impl<ValType, StopType> Context for StknDrop<ValType, StopType>
where
    ValType: DAMType + std::cmp::PartialEq + std::cmp::PartialOrd,
    StopType: DAMType + std::ops::Add<u32, Output = StopType> + std::cmp::PartialEq,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        let mut prev_stkn = false;
        loop {
            let val_deq = dequeue(&mut self.time, &mut self.in_val);
            match val_deq {
                Ok(curr_in) => match curr_in.data {
                    tkn @ Token::Val(_) | tkn @ Token::Done => {
                        let channel_elem = ChannelElement::new(self.time.tick() + 1, tkn.clone());
                        enqueue(&mut self.time, &mut self.out_val, channel_elem).unwrap();
                        if tkn == Token::Done {
                            return;
                        }
                        prev_stkn = false;
                    }
                    Token::Stop(stkn) => {
                        if !prev_stkn {
                            let channel_elem = ChannelElement::new(
                                self.time.tick() + 1,
                                Token::<ValType, StopType>::Stop(stkn),
                            );
                            enqueue(&mut self.time, &mut self.out_val, channel_elem).unwrap();
                            prev_stkn = true;
                        }
                    }
                    _ => {
                        panic!("Invalid token found in stream");
                    }
                },
                Err(_) => {
                    panic!("Unexpected end of stream");
                }
            }
            self.time.incr_cycles(1);
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.in_val.cleanup();
        self.out_val.cleanup();
        self.time.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        context::{checker_context::CheckerContext, generator_context::GeneratorContext},
        simulation::Program,
        templates::sam::primitive::Token,
        token_vec,
    };

    use super::StknDrop;

    #[test]
    fn val_drop_2d_test() {
        let in_val = || {
            token_vec![f32; u32; 0.0, 1.0, 2.0, "S0", 0.0, "S0", 2.0, 3.0, 4.0, "S1", "D"]
                .into_iter()
        };
        let in_crd =
            || token_vec![u32; u32; 0, 1, 2, "S0", 0, "S0", 2, 3, 4, "S1", "D"].into_iter();
        let out_val = || token_vec![f32; u32; 1.0, 2.0, "S0", 2.0, 3.0, 4.0, "S1", "D"].into_iter();
        let out_crd = || token_vec![u32; u32; 1, 2, "S0", 2, 3, 4, "S1", "D"].into_iter();
        // stkn_drop_test(in_val, out_val);
    }

    fn stkn_drop_test<IRT1, IRT2, ORT1, ORT2>(in_val: fn() -> IRT1, out_val: fn() -> ORT1)
    where
        IRT1: Iterator<Item = Token<f32, u32>> + 'static,
        ORT1: Iterator<Item = Token<f32, u32>> + 'static,
    {
        let mut parent = Program::default();
        let (in_val_sender, in_val_receiver) = parent.unbounded::<Token<f32, u32>>();
        let (out_val_sender, out_val_receiver) = parent.unbounded::<Token<f32, u32>>();
        let stkn_drop = StknDrop::new(in_val_receiver, out_val_sender);
        let gen1 = GeneratorContext::new(in_val, in_val_sender);
        let out_val_checker = CheckerContext::new(out_val, out_val_receiver);
        parent.add_child(gen1);
        parent.add_child(out_val_checker);
        parent.add_child(stkn_drop);
        parent.init();
        parent.run();
    }
}
