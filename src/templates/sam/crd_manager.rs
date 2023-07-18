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

pub struct CrdManagerData<ValType, StopType> {
    pub in_crd_inner: Receiver<Token<ValType, StopType>>,
    pub in_crd_outer: Receiver<Token<ValType, StopType>>,
    pub out_crd_inner: Sender<Token<ValType, StopType>>,
    pub out_crd_outer: Sender<Token<ValType, StopType>>,
}

impl<ValType: DAMType, StopType: DAMType> Cleanable for CrdManagerData<ValType, StopType> {
    fn cleanup(&mut self) {
        self.in_crd_inner.cleanup();
        self.in_crd_outer.cleanup();
        self.out_crd_inner.cleanup();
        self.out_crd_outer.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct CrdDrop<ValType, StopType> {
    crd_drop_data: CrdManagerData<ValType, StopType>,
}

impl<ValType: DAMType, StopType: DAMType> CrdDrop<ValType, StopType>
where
    CrdDrop<ValType, StopType>: Context,
{
    pub fn new(crd_drop_data: CrdManagerData<ValType, StopType>) -> Self {
        let drop = CrdDrop {
            crd_drop_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (drop.crd_drop_data.in_crd_inner).attach_receiver(&drop);
        (drop.crd_drop_data.in_crd_outer).attach_receiver(&drop);
        (drop.crd_drop_data.out_crd_inner).attach_sender(&drop);
        (drop.crd_drop_data.out_crd_outer).attach_sender(&drop);

        drop
    }
}

impl<ValType, StopType> Context for CrdDrop<ValType, StopType>
where
    ValType: DAMType
        + std::ops::Mul<ValType, Output = ValType>
        + std::ops::Add<ValType, Output = ValType>
        + std::cmp::PartialOrd<ValType>,
    StopType: DAMType + std::ops::Add<u32, Output = StopType>,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        // let mut get_next_ocrd = false;
        let mut has_crd = false;
        // let icrd_vec: Vec<Token<ValType, StopType>> = Vec::new();
        loop {
            let out_ocrd = peek_next(&mut self.time, &mut self.crd_drop_data.in_crd_outer);
            match dequeue(&mut self.time, &mut self.crd_drop_data.in_crd_inner) {
                Ok(curr_in) => {
                    let curr_ocrd = out_ocrd.unwrap().data.clone();
                    // dbg!(curr_in.data.clone());
                    // dbg!(curr_ocrd.clone());
                    let in_channel_elem =
                        ChannelElement::new(self.time.tick() + 1, curr_in.data.clone());
                    enqueue(
                        &mut self.time,
                        &mut self.crd_drop_data.out_crd_inner,
                        in_channel_elem,
                    )
                    .unwrap();
                    match curr_ocrd.clone() {
                        Token::Stop(tkn) => {
                            let channel_elem =
                                ChannelElement::new(self.time.tick() + 1, Token::Stop(tkn.clone()));
                            enqueue(
                                &mut self.time,
                                &mut self.crd_drop_data.out_crd_outer,
                                channel_elem,
                            )
                            .unwrap();
                            dequeue(&mut self.time, &mut self.crd_drop_data.in_crd_outer).unwrap();
                        }
                        _ => (),
                    }
                    match curr_in.data {
                        Token::Val(_) => {
                            has_crd = true;
                            continue;
                        }
                        Token::Stop(_) => {
                            if has_crd {
                                let channel_elem =
                                    ChannelElement::new(self.time.tick() + 1, curr_ocrd.clone());
                                enqueue(
                                    &mut self.time,
                                    &mut self.crd_drop_data.out_crd_outer,
                                    channel_elem,
                                )
                                .unwrap();
                                has_crd = false;
                                dequeue(&mut self.time, &mut self.crd_drop_data.in_crd_outer)
                                    .unwrap();
                                continue;
                            }

                            match curr_ocrd.clone() {
                                Token::Val(_) => {
                                    dequeue(&mut self.time, &mut self.crd_drop_data.in_crd_outer)
                                        .unwrap();
                                    has_crd = false;
                                    continue;
                                }
                                Token::Stop(tkn) => {
                                    let channel_elem = ChannelElement::new(
                                        self.time.tick() + 1,
                                        Token::Stop(tkn.clone()),
                                    );
                                    enqueue(
                                        &mut self.time,
                                        &mut self.crd_drop_data.out_crd_outer,
                                        channel_elem,
                                    )
                                    .unwrap();
                                    dequeue(&mut self.time, &mut self.crd_drop_data.in_crd_outer)
                                        .unwrap();
                                    has_crd = false;
                                }
                                _ => {
                                    panic!("Invalid empty or done token found in crdDrop");
                                }
                            }
                        }
                        Token::Done => {
                            // dbg!(curr_ocrd);
                            let ocrd =
                                dequeue(&mut self.time, &mut self.crd_drop_data.in_crd_outer)
                                    .unwrap();
                            match ocrd.data {
                                Token::Done => {
                                    let channel_elem =
                                        ChannelElement::new(self.time.tick() + 1, Token::Done);
                                    enqueue(
                                        &mut self.time,
                                        &mut self.crd_drop_data.out_crd_outer,
                                        channel_elem.clone(),
                                    )
                                    .unwrap();
                                    enqueue(
                                        &mut self.time,
                                        &mut self.crd_drop_data.out_crd_inner,
                                        channel_elem,
                                    )
                                    .unwrap();
                                    return;
                                }
                                _ => {
                                    panic!("Out crd should be done token");
                                }
                            }
                        }
                        _ => {
                            panic!("Invalid token reached");
                        }
                    }
                }
                Err(_) => {
                    panic!("Unexpected end of stream");
                }
            }
            self.time.incr_cycles(1);
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.crd_drop_data.cleanup();
        self.time.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct CrdHold<ValType, StopType> {
    crd_hold_data: CrdManagerData<ValType, StopType>,
}

impl<ValType: DAMType, StopType: DAMType> CrdHold<ValType, StopType>
where
    CrdHold<ValType, StopType>: Context,
{
    pub fn new(crd_hold_data: CrdManagerData<ValType, StopType>) -> Self {
        let hold = CrdHold {
            crd_hold_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (hold.crd_hold_data.in_crd_inner).attach_receiver(&hold);
        (hold.crd_hold_data.in_crd_outer).attach_receiver(&hold);
        (hold.crd_hold_data.out_crd_inner).attach_sender(&hold);
        (hold.crd_hold_data.out_crd_outer).attach_sender(&hold);

        hold
    }
}

impl<ValType, StopType> Context for CrdHold<ValType, StopType>
where
    ValType: DAMType
        + std::ops::Mul<ValType, Output = ValType>
        + std::ops::Add<ValType, Output = ValType>
        + std::cmp::PartialOrd<ValType>,
    StopType: DAMType + std::ops::Add<u32, Output = StopType>,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            let out_ocrd = peek_next(&mut self.time, &mut self.crd_hold_data.in_crd_outer);
            match dequeue(&mut self.time, &mut self.crd_hold_data.in_crd_inner) {
                Ok(curr_in) => {
                    let curr_ocrd = out_ocrd.unwrap().data.clone();
                    // dbg!(curr_in.data.clone());
                    // dbg!(curr_ocrd.clone());
                    let in_channel_elem =
                        ChannelElement::new(self.time.tick() + 1, curr_in.data.clone());
                    enqueue(
                        &mut self.time,
                        &mut self.crd_hold_data.out_crd_inner,
                        in_channel_elem,
                    )
                    .unwrap();

                    match curr_in.data.clone() {
                        Token::Val(_) => {
                            let output = match curr_ocrd.clone() {
                                Token::Val(_) => curr_ocrd.clone(),
                                Token::Stop(_) => {
                                    dequeue(&mut self.time, &mut self.crd_hold_data.in_crd_outer)
                                        .unwrap();
                                    peek_next(&mut self.time, &mut self.crd_hold_data.in_crd_outer)
                                        .unwrap()
                                        .data
                                }
                                _ => {
                                    panic!("Invalid token in output");
                                }
                            };
                            let channel_elem =
                                ChannelElement::new(self.time.tick() + 1, output.clone());
                            enqueue(
                                &mut self.time,
                                &mut self.crd_hold_data.out_crd_outer,
                                channel_elem,
                            )
                            .unwrap();
                        }
                        Token::Stop(_) => {
                            let channel_elem =
                                ChannelElement::new(self.time.tick() + 1, curr_in.data.clone());
                            enqueue(
                                &mut self.time,
                                &mut self.crd_hold_data.out_crd_outer,
                                channel_elem,
                            )
                            .unwrap();
                            dequeue(&mut self.time, &mut self.crd_hold_data.in_crd_outer).unwrap();
                        }
                        Token::Empty => todo!(),
                        tkn @ Token::Done => {
                            let channel_elem = ChannelElement::new(self.time.tick() + 1, tkn);
                            enqueue(
                                &mut self.time,
                                &mut self.crd_hold_data.out_crd_outer,
                                channel_elem,
                            )
                            .unwrap();
                            return;
                        }
                    }
                }
                Err(_) => {
                    panic!("Unexpected end of stream");
                }
            }
            self.time.incr_cycles(1);
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.crd_hold_data.cleanup();
        self.time.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        channel::unbounded,
        context::{
            checker_context::CheckerContext, generator_context::GeneratorContext,
            parent::BasicParentContext, Context,
        },
        templates::sam::primitive::Token,
        token_vec,
    };

    use super::CrdManagerData;
    use super::{CrdDrop, CrdHold};

    #[test]
    fn crd_drop_1d_test() {
        let in_ocrd = || token_vec!(u32; u32; 0, 1, "S0", "D").into_iter();
        let in_icrd = || token_vec!(u32; u32; 1, "S0", "S1", "D").into_iter();
        let out_ocrd = || token_vec!(u32; u32; 0, "S0", "D").into_iter();
        crd_drop_test(in_ocrd, in_icrd, out_ocrd);
    }

    #[test]
    fn crd_drop_1d_test1() {
        let in_ocrd = || token_vec!(u32; u32; 0, 1, 2, 3, "S0", "D").into_iter();
        let in_icrd = || token_vec!(u32; u32; 1, "S0", 1, "S0", "S0", 1, "S1", "D").into_iter();
        let out_ocrd = || token_vec!(u32; u32; 0, 1, 3, "S0", "D").into_iter();
        crd_drop_test(in_ocrd, in_icrd, out_ocrd);
    }

    #[test]
    fn crd_drop_1d_test2() {
        let in_ocrd = || token_vec!(u32; u32; 1, "S0", "D").into_iter();
        let in_icrd = || token_vec!(u32; u32; 1, 2, "S1", "D").into_iter();
        let out_ocrd = || token_vec!(u32; u32; 1, "S0", "D").into_iter();
        crd_drop_test(in_ocrd, in_icrd, out_ocrd);
    }

    #[test]
    fn crd_drop_1d_test3() {
        let in_ocrd = || token_vec!(u32; u32; 0, 1, "S0", "D").into_iter();
        let in_icrd = || token_vec!(u32; u32; 1, "S0", 1, "S1", "D").into_iter();
        let out_ocrd = || token_vec!(u32; u32; 0, 1, "S0", "D").into_iter();
        crd_drop_test(in_ocrd, in_icrd, out_ocrd);
    }

    #[test]
    fn crd_hold_1d_test() {
        let in_ocrd = || token_vec!(u32; u32; 0, 1, 2, "S0", "D").into_iter();
        let in_icrd = || token_vec!(u32; u32; 0, 2, "S0", 2, "S0", 2, "S1", "D").into_iter();
        let out_ocrd = || token_vec!(u32; u32; 0, 0, "S0", 1, "S0", 2, "S1", "D").into_iter();
        crd_hold_test(in_ocrd, in_icrd, out_ocrd);
    }

    #[test]
    fn crd_hold_1d_test1() {
        let in_ocrd = || token_vec!(u32; u32; 0, 2, "S0", 3, "S0", 4, "S1", "D").into_iter();
        let in_icrd = || {
            token_vec!(u32; u32; 0, 2, 3, "S0", 0, 2, 3, "S1", 0, "S1", 2, 3, "S2", "D").into_iter()
        };
        let out_ocrd = || {
            token_vec!(u32; u32; 0, 0, 0, "S0", 2, 2, 2, "S1", 3, "S1", 4, 4, "S2", "D").into_iter()
        };
        crd_hold_test(in_ocrd, in_icrd, out_ocrd);
    }

    #[test]
    fn crd_hold_1d_test2() {
        let in_ocrd = || token_vec!(u32; u32; 0, 1, 2, 5, "S0", "D").into_iter();
        let in_icrd = || {
            token_vec!(u32; u32; 1, 2, 5, "S0", 2, "S0", 2, "S0", 2, 3, 4, 5, "S1", "D").into_iter()
        };
        let out_ocrd = || {
            token_vec!(u32; u32; 0, 0, 0, "S0", 1, "S0", 2, "S0", 5, 5, 5, 5, "S1", "D").into_iter()
        };
        crd_hold_test(in_ocrd, in_icrd, out_ocrd);
    }

    fn crd_drop_test<IRT1, IRT2, ORT>(
        in_ocrd: fn() -> IRT1,
        in_icrd: fn() -> IRT2,
        out_ocrd: fn() -> ORT,
    ) where
        IRT1: Iterator<Item = Token<u32, u32>> + 'static,
        IRT2: Iterator<Item = Token<u32, u32>> + 'static,
        ORT: Iterator<Item = Token<u32, u32>> + 'static,
    {
        let (in_ocrd_sender, in_ocrd_receiver) = unbounded::<Token<u32, u32>>();
        let (in_icrd_sender, in_icrd_receiver) = unbounded::<Token<u32, u32>>();
        let (out_ocrd_sender, out_ocrd_receiver) = unbounded::<Token<u32, u32>>();
        let (out_icrd_sender, _out_icrd_receiver) = unbounded::<Token<u32, u32>>();

        let crd_drop_data = CrdManagerData::<u32, u32> {
            in_crd_outer: in_ocrd_receiver,
            in_crd_inner: in_icrd_receiver,
            out_crd_outer: out_ocrd_sender,
            out_crd_inner: out_icrd_sender,
        };

        let mut drop = CrdDrop::new(crd_drop_data);
        let mut ocrd_gen = GeneratorContext::new(in_ocrd, in_ocrd_sender);
        let mut icrd_gen = GeneratorContext::new(in_icrd, in_icrd_sender);
        let mut out_crd_checker = CheckerContext::new(out_ocrd, out_ocrd_receiver);
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut ocrd_gen);
        parent.add_child(&mut icrd_gen);
        parent.add_child(&mut out_crd_checker);
        parent.add_child(&mut drop);
        parent.init();
        parent.run();
        parent.cleanup();
    }

    fn crd_hold_test<IRT1, IRT2, ORT>(
        in_ocrd: fn() -> IRT1,
        in_icrd: fn() -> IRT2,
        out_ocrd: fn() -> ORT,
    ) where
        IRT1: Iterator<Item = Token<u32, u32>> + 'static,
        IRT2: Iterator<Item = Token<u32, u32>> + 'static,
        ORT: Iterator<Item = Token<u32, u32>> + 'static,
    {
        let (in_ocrd_sender, in_ocrd_receiver) = unbounded::<Token<u32, u32>>();
        let (in_icrd_sender, in_icrd_receiver) = unbounded::<Token<u32, u32>>();
        let (out_ocrd_sender, out_ocrd_receiver) = unbounded::<Token<u32, u32>>();
        let (out_icrd_sender, _out_icrd_receiver) = unbounded::<Token<u32, u32>>();

        let crd_hold_data = CrdManagerData::<u32, u32> {
            in_crd_outer: in_ocrd_receiver,
            in_crd_inner: in_icrd_receiver,
            out_crd_outer: out_ocrd_sender,
            out_crd_inner: out_icrd_sender,
        };

        let mut drop = CrdHold::new(crd_hold_data);
        let mut ocrd_gen = GeneratorContext::new(in_ocrd, in_ocrd_sender);
        let mut icrd_gen = GeneratorContext::new(in_icrd, in_icrd_sender);
        let mut out_crd_checker = CheckerContext::new(out_ocrd, out_ocrd_receiver);
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut ocrd_gen);
        parent.add_child(&mut icrd_gen);
        parent.add_child(&mut out_crd_checker);
        parent.add_child(&mut drop);
        parent.init();
        parent.run();
        parent.cleanup();
    }
}
