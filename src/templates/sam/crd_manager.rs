use core::panic;

use crate::{
    channel::{
        utils::{dequeue, enqueue, peek_next},
        ChannelElement, Receiver, Sender,
    },
    context::{
        view::{TimeManager, TimeView},
        Context,
    },
    types::{Cleanable, DAMType},
};

use super::primitive::Token;

pub struct CrdDropData<ValType, StopType> {
    pub in_crd_inner: Receiver<Token<ValType, StopType>>,
    pub in_crd_outer: Receiver<Token<ValType, StopType>>,
    pub out_crd_inner: Sender<Token<ValType, StopType>>,
    pub out_crd_outer: Sender<Token<ValType, StopType>>,
}

impl<ValType: DAMType, StopType: DAMType> Cleanable for CrdDropData<ValType, StopType> {
    fn cleanup(&mut self) {
        self.in_crd_inner.cleanup();
        self.in_crd_outer.cleanup();
        self.out_crd_inner.cleanup();
        self.out_crd_outer.cleanup();
    }
}

pub struct CrdDrop<ValType, StopType> {
    crd_drop_data: CrdDropData<ValType, StopType>,
    time: TimeManager,
}

impl<ValType: DAMType, StopType: DAMType> CrdDrop<ValType, StopType>
where
    CrdDrop<ValType, StopType>: Context,
{
    pub fn new(crd_drop_data: CrdDropData<ValType, StopType>) -> Self {
        let drop = CrdDrop {
            crd_drop_data,
            time: TimeManager::default(),
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
        let mut ocrd_vec: Vec<Token<ValType, StopType>> = Vec::new();
        // let icrd_vec: Vec<Token<ValType, StopType>> = Vec::new();
        loop {
            // if get_next_ocrd {
            //     dequeue(&mut self.time, &mut self.crd_drop_data.in_crd_outer);
            // }
            let out_ocrd = peek_next(&mut self.time, &mut self.crd_drop_data.in_crd_outer);
            match dequeue(&mut self.time, &mut self.crd_drop_data.in_crd_inner) {
                Ok(curr_in) => {
                    let curr_ocrd = out_ocrd.unwrap().data.clone();
                    // dbg!(curr_in.data.clone());
                    let in_channel_elem =
                        ChannelElement::new(self.time.tick() + 1, curr_in.data.clone());
                    enqueue(
                        &mut self.time,
                        &mut self.crd_drop_data.out_crd_inner,
                        in_channel_elem,
                    )
                    .unwrap();
                    match curr_in.data {
                        Token::Val(_) => {
                            has_crd = true;
                            continue;
                        }
                        Token::Stop(stkn) => {
                            // dbg!(Token::<ValType, StopType>::Stop(stkn));
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
                                ocrd_vec.push(curr_ocrd.clone());
                                dequeue(&mut self.time, &mut self.crd_drop_data.in_crd_outer)
                                    .unwrap();
                                continue;
                            }
                            // let curr_ocrd =
                            //     dequeue(&mut self.time, &mut self.crd_drop_data.in_crd_outer)
                            //         .unwrap();
                            // let curr_ocrd =
                            match curr_ocrd.clone() {
                                Token::Val(_) => {
                                    // dequeue(&mut self.time, &mut self.crd_drop_data.in_crd_outer)
                                    //     .unwrap();
                                    continue;
                                }
                                Token::Stop(tkn) => {
                                    let channel_elem = ChannelElement::new(
                                        self.time.tick() + 1,
                                        Token::Stop(tkn.clone()),
                                    );
                                    ocrd_vec.push(Token::Stop(tkn.clone()));
                                    enqueue(
                                        &mut self.time,
                                        &mut self.crd_drop_data.out_crd_outer,
                                        channel_elem,
                                    )
                                    .unwrap();
                                    dequeue(&mut self.time, &mut self.crd_drop_data.in_crd_outer)
                                        .unwrap();
                                    // dbg!("Pushing", Token::<ValType, StopType>::Stop(tkn.clone()));
                                }
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
                                    dbg!(Token::<ValType, StopType>::Done);
                                    return;
                                }
                                _ => {
                                    panic!("Invalid empty token found in crdDrop");
                                }
                            }
                            has_crd = false;
                        }
                        Token::Done => {
                            // dbg!(curr_ocrd);
                            let ocrd =
                                dequeue(&mut self.time, &mut self.crd_drop_data.in_crd_outer)
                                    .unwrap();
                            dbg!(ocrd.data.clone());
                            ocrd_vec.push(Token::Done);
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

    fn cleanup(&mut self) {
        self.crd_drop_data.cleanup();
        self.time.cleanup();
    }

    fn view(&self) -> TimeView {
        self.time.view().into()
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        channel::unbounded,
        context::{
            checker_context::CheckerContext, generator_context::GeneratorContext,
            parent::BasicParentContext, Context, ParentContext,
        },
        templates::sam::primitive::Token,
        token_vec,
    };

    use super::CrdDrop;
    use super::CrdDropData;

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
        let (out_icrd_sender, out_icrd_receiver) = unbounded::<Token<u32, u32>>();

        let crd_drop_data = CrdDropData::<u32, u32> {
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
}
