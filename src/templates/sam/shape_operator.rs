use core::panic;

use dam_core::{identifier::Identifier, TimeManager};
use dam_macros::{cleanup, identifiable, time_managed};

use crate::{
    channel::{
        utils::{dequeue, enqueue, peek_next},
        ChannelElement, Receiver, Sender,
    },
    context::Context,
    types::{Cleanable, DAMType},
};

use super::primitive::Token;

pub struct FlattenData<ValType: Clone, StopType: Clone> {
    pub in_crd_inner: Receiver<Token<ValType, StopType>>,
    pub in_crd_outer: Receiver<Token<ValType, StopType>>,
    pub out_crd: Sender<Token<ValType, StopType>>,
}

impl<ValType: DAMType, StopType: DAMType> Cleanable for FlattenData<ValType, StopType> {
    fn cleanup(&mut self) {
        self.in_crd_inner.cleanup();
        self.in_crd_outer.cleanup();
        self.out_crd.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct Flatten<ValType: Clone, StopType: Clone> {
    flatten_data: FlattenData<ValType, StopType>,
    split_factor: u32,
}

impl<ValType: DAMType, StopType: DAMType> Flatten<ValType, StopType>
where
    Flatten<ValType, StopType>: Context,
{
    pub fn new(flatten_data: FlattenData<ValType, StopType>, split_factor: u32) -> Self {
        let flat = Flatten {
            flatten_data,
            split_factor,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (flat.flatten_data.in_crd_inner).attach_receiver(&flat);
        (flat.flatten_data.in_crd_outer).attach_receiver(&flat);
        (flat.flatten_data.out_crd).attach_sender(&flat);

        flat
    }
}

impl<ValType, StopType> Context for Flatten<ValType, StopType>
where
    ValType: DAMType
        + std::ops::Mul<ValType, Output = ValType>
        + std::ops::Mul<u32, Output = ValType>
        + std::ops::Add<ValType, Output = ValType>
        + std::cmp::PartialOrd<ValType>,
    StopType: DAMType + std::ops::Add<u32, Output = StopType>,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            let out_ocrd = peek_next(&mut self.time, &mut self.flatten_data.in_crd_outer);
            match dequeue(&mut self.time, &mut self.flatten_data.in_crd_inner) {
                Ok(curr_in) => {
                    let curr_ocrd = out_ocrd.unwrap().data.clone();
                    match curr_in.data {
                        Token::Val(icrd) => {
                            match curr_ocrd.clone() {
                                Token::Val(ocrd) => {
                                    let new_crd = ocrd * self.split_factor + icrd;
                                    let channel_elem = ChannelElement::new(
                                        self.time.tick() + 1,
                                        Token::<ValType, StopType>::Val(new_crd),
                                    );
                                    enqueue(
                                        &mut self.time,
                                        &mut self.flatten_data.out_crd,
                                        channel_elem,
                                    )
                                    .unwrap();
                                }
                                _ => {
                                    panic!("Unexpected case found, found val icrd and control token ocrd");
                                }
                            }
                        }
                        Token::Stop(_) => {
                            match curr_ocrd.clone() {
                                Token::Stop(stkn) => {
                                    let channel_elem = ChannelElement::new(
                                        self.time.tick() + 1,
                                        Token::<ValType, StopType>::Stop(stkn),
                                    );
                                    enqueue(
                                        &mut self.time,
                                        &mut self.flatten_data.out_crd,
                                        channel_elem,
                                    )
                                    .unwrap();
                                }
                                _ => (), // _ => {
                                         // panic!("Should be a stop token for ocrd");
                                         // }
                            }
                            dequeue(&mut self.time, &mut self.flatten_data.in_crd_outer).unwrap();
                        }
                        Token::Done => {
                            let channel_elem =
                                ChannelElement::new(self.time.tick() + 1, Token::Done);
                            enqueue(&mut self.time, &mut self.flatten_data.out_crd, channel_elem)
                                .unwrap();
                            return;
                        }
                        _ => {
                            panic!("Empty token found in shape operator");
                        }
                    }
                }
                Err(_) => todo!(),
            }
            self.time.incr_cycles(1);
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.flatten_data.cleanup();
        self.time.cleanup();
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

    use super::Flatten;
    use super::FlattenData;

    #[test]
    fn flatten_2d_test() {
        let in_ocrd = || token_vec!(u32; u32; 0, 2, 3, "S0", "D").into_iter();
        let in_icrd =
            || token_vec!(u32; u32; 0, 2, 3, "S0", 9, 11, "S0", 12, "S1", "D").into_iter();
        let out_ocrd = || token_vec!(u32; u32; 0, 2, 3, 9, 11, 12, "S0", "D").into_iter();
        flatten_test(in_ocrd, in_icrd, out_ocrd);
    }

    fn flatten_test<IRT1, IRT2, ORT>(
        in_ocrd: fn() -> IRT1,
        in_icrd: fn() -> IRT2,
        out_crd: fn() -> ORT,
    ) where
        IRT1: Iterator<Item = Token<u32, u32>> + 'static,
        IRT2: Iterator<Item = Token<u32, u32>> + 'static,
        ORT: Iterator<Item = Token<u32, u32>> + 'static,
    {
        let mut parent = Program::default();
        let (in_ocrd_sender, in_ocrd_receiver) = parent.unbounded::<Token<u32, u32>>();
        let (in_icrd_sender, in_icrd_receiver) = parent.unbounded::<Token<u32, u32>>();
        let (out_crd_sender, out_crd_receiver) = parent.unbounded::<Token<u32, u32>>();

        let crd_drop_data = FlattenData::<u32, u32> {
            in_crd_outer: in_ocrd_receiver,
            in_crd_inner: in_icrd_receiver,
            out_crd: out_crd_sender,
        };

        let flat = Flatten::new(crd_drop_data, 4);
        let ocrd_gen = GeneratorContext::new(in_ocrd, in_ocrd_sender);
        let icrd_gen = GeneratorContext::new(in_icrd, in_icrd_sender);
        let out_crd_checker = CheckerContext::new(out_crd, out_crd_receiver);

        parent.add_child(ocrd_gen);
        parent.add_child(icrd_gen);
        parent.add_child(out_crd_checker);
        parent.add_child(flat);
        parent.init();
        parent.run();
    }
}
