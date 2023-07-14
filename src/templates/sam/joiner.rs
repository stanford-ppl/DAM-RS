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

pub struct CrdJoinerData<ValType, StopType> {
    pub in_crd1: Receiver<Token<ValType, StopType>>,
    pub in_ref1: Receiver<Token<ValType, StopType>>,
    pub in_crd2: Receiver<Token<ValType, StopType>>,
    pub in_ref2: Receiver<Token<ValType, StopType>>,
    pub out_ref1: Sender<Token<ValType, StopType>>,
    pub out_ref2: Sender<Token<ValType, StopType>>,
    pub out_crd: Sender<Token<ValType, StopType>>,
}

impl<ValType: DAMType, StopType: DAMType> Cleanable for CrdJoinerData<ValType, StopType> {
    fn cleanup(&mut self) {
        self.in_crd1.cleanup();
        self.in_ref1.cleanup();
        self.in_crd2.cleanup();
        self.in_ref2.cleanup();
        self.out_ref1.cleanup();
        self.out_ref2.cleanup();
        self.out_crd.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct Intersect<ValType, StopType> {
    intersect_data: CrdJoinerData<ValType, StopType>,
}

impl<ValType: DAMType, StopType: DAMType> Intersect<ValType, StopType>
where
    Intersect<ValType, StopType>: Context,
{
    pub fn new(intersect_data: CrdJoinerData<ValType, StopType>) -> Self {
        let int = Intersect {
            intersect_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (int.intersect_data.in_crd1).attach_receiver(&int);
        (int.intersect_data.in_ref1).attach_receiver(&int);
        (int.intersect_data.in_crd2).attach_receiver(&int);
        (int.intersect_data.in_ref2).attach_receiver(&int);
        (int.intersect_data.out_ref1).attach_sender(&int);
        (int.intersect_data.out_ref2).attach_sender(&int);
        (int.intersect_data.out_crd).attach_sender(&int);

        int
    }
}

impl<ValType, StopType> Context for Intersect<ValType, StopType>
where
    ValType: DAMType
        + std::ops::AddAssign<u32>
        + std::ops::AddAssign<ValType>
        + std::ops::Mul<ValType, Output = ValType>
        + std::ops::Add<ValType, Output = ValType>
        + std::cmp::PartialOrd<ValType>,
    StopType: DAMType
        + std::ops::Add<u32, Output = StopType>
        + std::ops::Sub<u32, Output = StopType>
        + std::cmp::PartialEq,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        let mut get_crd1: bool = false;
        let mut get_crd2: bool = false;

        loop {
            if get_crd1 == true {
                dequeue(&mut self.time, &mut self.intersect_data.in_crd1).unwrap();
                dequeue(&mut self.time, &mut self.intersect_data.in_ref1).unwrap();
            }
            if get_crd2 == true {
                dequeue(&mut self.time, &mut self.intersect_data.in_crd2).unwrap();
                dequeue(&mut self.time, &mut self.intersect_data.in_ref2).unwrap();
            }
            let crd1_deq = peek_next(&mut self.time, &mut self.intersect_data.in_crd1);
            let crd2_deq = peek_next(&mut self.time, &mut self.intersect_data.in_crd2);
            let ref1_deq = peek_next(&mut self.time, &mut self.intersect_data.in_ref1);
            let ref2_deq = peek_next(&mut self.time, &mut self.intersect_data.in_ref2);

            match (crd1_deq, crd2_deq) {
                (Ok(crd1), Ok(crd2)) => {
                    let ref1: Token<ValType, StopType> = ref1_deq.unwrap().data;
                    let ref2: Token<ValType, StopType> = ref2_deq.unwrap().data;
                    match (crd1.data, crd2.data) {
                        (Token::Val(crd1), Token::Val(crd2)) => match (crd1, crd2) {
                            (crd1, crd2) if crd1 == crd2 => {
                                let curr_time = self.time.tick();
                                enqueue(
                                    &mut self.time,
                                    &mut self.intersect_data.out_crd,
                                    ChannelElement::new(curr_time + 1, Token::Val(crd1)),
                                )
                                .unwrap();
                                enqueue(
                                    &mut self.time,
                                    &mut self.intersect_data.out_ref1,
                                    ChannelElement::new(curr_time + 1, ref1),
                                )
                                .unwrap();
                                enqueue(
                                    &mut self.time,
                                    &mut self.intersect_data.out_ref2,
                                    ChannelElement::new(curr_time + 1, ref2),
                                )
                                .unwrap();
                                get_crd1 = true;
                                get_crd2 = true;
                            }
                            (crd1, crd2) if crd1 < crd2 => {
                                get_crd1 = true;
                                get_crd2 = false;
                            }
                            (crd1, crd2) if crd1 > crd2 => {
                                get_crd1 = false;
                                get_crd2 = true;
                            }
                            (_, _) => {
                                panic!("Unexpected case found in val comparison");
                            }
                        },
                        (Token::Val(_), Token::Stop(_)) => {
                            get_crd1 = true;
                            get_crd2 = false;
                        }
                        (Token::Val(_), Token::Done) | (Token::Done, Token::Val(_)) => {
                            let curr_time = self.time.tick();
                            enqueue(
                                &mut self.time,
                                &mut self.intersect_data.out_crd,
                                ChannelElement::new(curr_time + 1, Token::Done),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.intersect_data.out_ref1,
                                ChannelElement::new(curr_time + 1, Token::Done),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.intersect_data.out_ref2,
                                ChannelElement::new(curr_time + 1, Token::Done),
                            )
                            .unwrap();
                        }
                        (Token::Stop(_), Token::Val(_)) => {
                            get_crd1 = false;
                            get_crd2 = true;
                        }
                        (Token::Stop(stkn1), Token::Stop(_)) => {
                            let curr_time = self.time.tick();
                            enqueue(
                                &mut self.time,
                                &mut self.intersect_data.out_crd,
                                ChannelElement::new(curr_time + 1, Token::Stop(stkn1)),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.intersect_data.out_ref1,
                                ChannelElement::new(curr_time + 1, ref1),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.intersect_data.out_ref2,
                                ChannelElement::new(curr_time + 1, ref2),
                            )
                            .unwrap();
                            get_crd1 = true;
                            get_crd2 = true;
                        }
                        (tkn @ Token::Empty, Token::Val(_))
                        | (Token::Val(_), tkn @ Token::Empty)
                        | (tkn @ Token::Done, Token::Done) => {
                            let channel_elem =
                                ChannelElement::new(self.time.tick() + 1, tkn.clone());
                            enqueue(
                                &mut self.time,
                                &mut self.intersect_data.out_crd,
                                channel_elem.clone(),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.intersect_data.out_ref1,
                                channel_elem.clone(),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.intersect_data.out_ref2,
                                channel_elem.clone(),
                            )
                            .unwrap();
                            if tkn.clone() == Token::Done {
                                return;
                            }
                        }
                        _ => (),
                    }
                }
                (_, _) => {
                    panic!("Reached unhandled case");
                }
            }
            self.time.incr_cycles(1);
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.intersect_data.cleanup();
        self.time.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct Union<ValType, StopType> {
    union_data: CrdJoinerData<ValType, StopType>,
}

impl<ValType: DAMType, StopType: DAMType> Union<ValType, StopType>
where
    Union<ValType, StopType>: Context,
{
    pub fn new(union_data: CrdJoinerData<ValType, StopType>) -> Self {
        let int = Union {
            union_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (int.union_data.in_crd1).attach_receiver(&int);
        (int.union_data.in_ref1).attach_receiver(&int);
        (int.union_data.in_crd2).attach_receiver(&int);
        (int.union_data.in_ref2).attach_receiver(&int);
        (int.union_data.out_ref1).attach_sender(&int);
        (int.union_data.out_ref2).attach_sender(&int);
        (int.union_data.out_crd).attach_sender(&int);

        int
    }
}

impl<ValType, StopType> Context for Union<ValType, StopType>
where
    ValType: DAMType
        + std::ops::AddAssign<u32>
        + std::ops::AddAssign<ValType>
        + std::ops::Mul<ValType, Output = ValType>
        + std::ops::Add<ValType, Output = ValType>
        + std::cmp::PartialOrd<ValType>,
    StopType:
        DAMType + std::ops::Add<u32, Output = StopType> + std::ops::Sub<u32, Output = StopType>,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        let mut get_crd1: bool = false;
        let mut get_crd2: bool = false;

        loop {
            if get_crd1 == true {
                dequeue(&mut self.time, &mut self.union_data.in_crd1).unwrap();
                dequeue(&mut self.time, &mut self.union_data.in_ref1).unwrap();
            }
            if get_crd2 == true {
                dequeue(&mut self.time, &mut self.union_data.in_crd2).unwrap();
                dequeue(&mut self.time, &mut self.union_data.in_ref2).unwrap();
            }
            let ref1_deq = peek_next(&mut self.time, &mut self.union_data.in_ref1);
            let ref2_deq = peek_next(&mut self.time, &mut self.union_data.in_ref2);
            let crd1_deq = peek_next(&mut self.time, &mut self.union_data.in_crd1);
            let crd2_deq = peek_next(&mut self.time, &mut self.union_data.in_crd2);

            match (crd1_deq, crd2_deq) {
                (Ok(crd1), Ok(crd2)) => {
                    let ref1: Token<ValType, StopType> = ref1_deq.unwrap().data;
                    let ref2: Token<ValType, StopType> = ref2_deq.unwrap().data;
                    let curr_time = self.time.tick();
                    match (crd1.data, crd2.data) {
                        (Token::Val(crd1), Token::Val(crd2)) => match (crd1, crd2) {
                            (crd1, crd2) if crd1 == crd2 => {
                                enqueue(
                                    &mut self.time,
                                    &mut self.union_data.out_crd,
                                    ChannelElement::new(curr_time + 1, Token::Val(crd1)),
                                )
                                .unwrap();
                                enqueue(
                                    &mut self.time,
                                    &mut self.union_data.out_ref1,
                                    ChannelElement::new(curr_time + 1, ref1),
                                )
                                .unwrap();
                                enqueue(
                                    &mut self.time,
                                    &mut self.union_data.out_ref2,
                                    ChannelElement::new(curr_time + 1, ref2),
                                )
                                .unwrap();
                                get_crd1 = true;
                                get_crd2 = true;
                            }
                            (crd1, crd2) if crd1 < crd2 => {
                                enqueue(
                                    &mut self.time,
                                    &mut self.union_data.out_crd,
                                    ChannelElement::new(curr_time + 1, Token::Val(crd1)),
                                )
                                .unwrap();
                                enqueue(
                                    &mut self.time,
                                    &mut self.union_data.out_ref1,
                                    ChannelElement::new(curr_time + 1, ref1),
                                )
                                .unwrap();
                                enqueue(
                                    &mut self.time,
                                    &mut self.union_data.out_ref2,
                                    ChannelElement::new(curr_time + 1, Token::Empty),
                                )
                                .unwrap();
                                get_crd1 = true;
                                get_crd2 = false;
                            }
                            (crd1, crd2) if crd1 > crd2 => {
                                enqueue(
                                    &mut self.time,
                                    &mut self.union_data.out_crd,
                                    ChannelElement::new(curr_time + 1, Token::Val(crd1)),
                                )
                                .unwrap();
                                enqueue(
                                    &mut self.time,
                                    &mut self.union_data.out_ref1,
                                    ChannelElement::new(curr_time + 1, Token::Empty),
                                )
                                .unwrap();
                                enqueue(
                                    &mut self.time,
                                    &mut self.union_data.out_ref2,
                                    ChannelElement::new(curr_time + 1, ref2),
                                )
                                .unwrap();
                                get_crd1 = false;
                                get_crd2 = true;
                            }
                            (_, _) => {
                                panic!("Unexpected case found in val comparison");
                            }
                        },
                        (Token::Val(crd1), Token::Stop(_)) | (Token::Val(crd1), Token::Empty) => {
                            enqueue(
                                &mut self.time,
                                &mut self.union_data.out_crd,
                                ChannelElement::new(curr_time + 1, Token::Val(crd1)),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.union_data.out_ref1,
                                ChannelElement::new(curr_time + 1, ref1),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.union_data.out_ref2,
                                ChannelElement::new(curr_time + 1, Token::Empty),
                            )
                            .unwrap();
                            get_crd1 = true;
                            get_crd2 = false;
                        }
                        (Token::Val(_), Token::Done)
                        | (Token::Done, Token::Val(_))
                        | (Token::Done, Token::Done) => {
                            let channel_elem =
                                ChannelElement::new(self.time.tick() + 1, Token::Done);
                            enqueue(
                                &mut self.time,
                                &mut self.union_data.out_crd,
                                channel_elem.clone(),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.union_data.out_ref1,
                                channel_elem.clone(),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.union_data.out_ref2,
                                channel_elem.clone(),
                            )
                            .unwrap();
                            return;
                        }
                        (Token::Stop(_), Token::Val(crd2)) | (Token::Empty, Token::Val(crd2)) => {
                            enqueue(
                                &mut self.time,
                                &mut self.union_data.out_crd,
                                ChannelElement::new(curr_time + 1, Token::Val(crd2)),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.union_data.out_ref1,
                                ChannelElement::new(curr_time + 1, Token::Empty),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.union_data.out_ref2,
                                ChannelElement::new(curr_time + 1, ref2),
                            )
                            .unwrap();
                            get_crd1 = false;
                            get_crd2 = true;
                        }
                        (Token::Stop(stkn1), Token::Stop(_)) => {
                            enqueue(
                                &mut self.time,
                                &mut self.union_data.out_crd,
                                ChannelElement::new(curr_time + 1, Token::Stop(stkn1)),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.union_data.out_ref1,
                                ChannelElement::new(curr_time + 1, ref1),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.union_data.out_ref2,
                                ChannelElement::new(curr_time + 1, ref2),
                            )
                            .unwrap();
                            get_crd1 = true;
                            get_crd2 = true;
                        }
                        _ => (),
                    }
                }
                (_, _) => {
                    panic!("Reached unhandled case");
                }
            }
            self.time.incr_cycles(1);
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.union_data.cleanup();
        self.time.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        channel::{bounded, unbounded},
        context::{
            checker_context::CheckerContext, generator_context::GeneratorContext,
            parent::BasicParentContext, Context, ParentContext,
        },
        templates::sam::primitive::Token,
        token_vec,
    };

    use super::CrdJoinerData;
    use super::Intersect;
    use super::Union;

    #[test]
    fn intersect_2d_test() {
        let in_crd1 = || token_vec!(u32; u32; 0, "S0", 0, 1, 2, "S1", "D").into_iter();
        let in_ref1 = || token_vec!(u32; u32; 0, "S0", 1, 2, 3, "S1", "D").into_iter();
        let in_crd2 = || token_vec!(u32; u32; 0, 1, 2, "S0", 0, 1, 2, "S1", "D").into_iter();
        let in_ref2 = || token_vec!(u32; u32; 0, 1, 2, "S0", 0, 1, 2, "S1", "D").into_iter();

        let out_crd = || token_vec!(u32; u32; 0, "S0", 0, 1, 2, "S1", "D").into_iter();
        let out_ref1 = || token_vec!(u32; u32; 0, "S0", 1, 2, 3, "S1", "D").into_iter();
        let out_ref2 = || token_vec!(u32; u32; 0, "S0", 0, 1, 2, "S1", "D").into_iter();
        // dbg!(token_vec!(u32; u32; 0, "S0", 0, 1, 2, "S1", "D"));
        intersect_test(
            in_crd1, in_ref1, in_crd2, in_ref2, out_crd, out_ref1, out_ref2,
        );
    }

    #[test]
    fn union_2d_test() {
        let in_crd1 =
            || token_vec!(u32; u32; 0, 1, "S0", 2, 3, "S0", "S0", 4, 5, "S1", "D").into_iter();
        let in_ref1 =
            || token_vec!(u32; u32; 0, 1, "S0", 2, 3, "S0", "S0", 4, 5, "S1", "D").into_iter();
        let in_crd2 =
            || token_vec!(u32; u32; 1, 2, 3, "S0", "S0", 0, 1, 2, "S0", "S1", "D").into_iter();
        let in_ref2 =
            || token_vec!(u32; u32; 0, 1, 2, "S0", "S0", 2, 3, 4, "S0", "S1", "D").into_iter();

        let out_crd = || {
            token_vec!(u32; u32; 0, 1, 2, 3, "S0", 2, 3, "S0", 0, 1, 2, "S0", 4, 5, "S1", "D")
                .into_iter()
        };
        let out_ref1 = || {
            token_vec!(u32; u32; 0, 1, "N", "N", "S0", 2, 3, "S0", "N", "N", "N", "S0", 4, 5, "S1", "D").into_iter()
        };
        let out_ref2 = || {
            token_vec!(u32; u32; "N", 0, 1, 2, "S0", "N", "N", "S0", 2, 3, 4, "S0", "N", "N", "S1", "D").into_iter()
        };
        // dbg!(token_vec!(u32; u32; 0, "S0", 0, 1, 2, "S1", "D"));
        union_test(
            in_crd1, in_ref1, in_crd2, in_ref2, out_crd, out_ref1, out_ref2,
        );
    }

    fn intersect_test<IRT1, IRT2, IRT3, IRT4, ORT1, ORT2, ORT3>(
        in_crd1: fn() -> IRT1,
        in_ref1: fn() -> IRT2,
        in_crd2: fn() -> IRT3,
        in_ref2: fn() -> IRT4,
        out_crd: fn() -> ORT1,
        out_ref1: fn() -> ORT2,
        out_ref2: fn() -> ORT3,
    ) where
        IRT1: Iterator<Item = Token<u32, u32>> + 'static,
        IRT2: Iterator<Item = Token<u32, u32>> + 'static,
        IRT3: Iterator<Item = Token<u32, u32>> + 'static,
        IRT4: Iterator<Item = Token<u32, u32>> + 'static,
        ORT1: Iterator<Item = Token<u32, u32>> + 'static,
        ORT2: Iterator<Item = Token<u32, u32>> + 'static,
        ORT3: Iterator<Item = Token<u32, u32>> + 'static,
    {
        let chan_size = 4;

        let (in_crd1_sender, in_crd1_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (in_crd2_sender, in_crd2_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (in_ref1_sender, in_ref1_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (in_ref2_sender, in_ref2_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (out_crd_sender, out_crd_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (out_ref1_sender, out_ref1_receiver) = bounded::<Token<u32, u32>>(chan_size);
        let (out_ref2_sender, out_ref2_receiver) = bounded::<Token<u32, u32>>(chan_size);

        let data = CrdJoinerData::<u32, u32> {
            in_crd1: in_crd1_receiver,
            in_ref1: in_ref1_receiver,
            in_crd2: in_crd2_receiver,
            in_ref2: in_ref2_receiver,
            out_crd: out_crd_sender,
            out_ref1: out_ref1_sender,
            out_ref2: out_ref2_sender,
        };
        let mut intersect = Intersect::new(data);
        let mut gen1 = GeneratorContext::new(in_crd1, in_crd1_sender);
        let mut gen2 = GeneratorContext::new(in_ref1, in_ref1_sender);
        let mut gen3 = GeneratorContext::new(in_crd2, in_crd2_sender);
        let mut gen4 = GeneratorContext::new(in_ref2, in_ref2_sender);
        let mut crd_checker = CheckerContext::new(out_crd, out_crd_receiver);
        let mut ref1_checker = CheckerContext::new(out_ref1, out_ref1_receiver);
        let mut ref2_checker = CheckerContext::new(out_ref2, out_ref2_receiver);
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut gen2);
        parent.add_child(&mut gen3);
        parent.add_child(&mut gen4);
        parent.add_child(&mut crd_checker);
        parent.add_child(&mut ref1_checker);
        parent.add_child(&mut ref2_checker);
        parent.add_child(&mut intersect);
        parent.init();
        parent.run();
        parent.cleanup();
    }

    fn union_test<IRT1, IRT2, IRT3, IRT4, ORT1, ORT2, ORT3>(
        in_crd1: fn() -> IRT1,
        in_ref1: fn() -> IRT2,
        in_crd2: fn() -> IRT3,
        in_ref2: fn() -> IRT4,
        out_crd: fn() -> ORT1,
        out_ref1: fn() -> ORT2,
        out_ref2: fn() -> ORT3,
    ) where
        IRT1: Iterator<Item = Token<u32, u32>> + 'static,
        IRT2: Iterator<Item = Token<u32, u32>> + 'static,
        IRT3: Iterator<Item = Token<u32, u32>> + 'static,
        IRT4: Iterator<Item = Token<u32, u32>> + 'static,
        ORT1: Iterator<Item = Token<u32, u32>> + 'static,
        ORT2: Iterator<Item = Token<u32, u32>> + 'static,
        ORT3: Iterator<Item = Token<u32, u32>> + 'static,
    {
        let (in_crd1_sender, in_crd1_receiver) = unbounded::<Token<u32, u32>>();
        let (in_crd2_sender, in_crd2_receiver) = unbounded::<Token<u32, u32>>();
        let (in_ref1_sender, in_ref1_receiver) = unbounded::<Token<u32, u32>>();
        let (in_ref2_sender, in_ref2_receiver) = unbounded::<Token<u32, u32>>();
        let (out_crd_sender, out_crd_receiver) = unbounded::<Token<u32, u32>>();
        let (out_ref1_sender, out_ref1_receiver) = unbounded::<Token<u32, u32>>();
        let (out_ref2_sender, out_ref2_receiver) = unbounded::<Token<u32, u32>>();

        let data = CrdJoinerData::<u32, u32> {
            in_crd1: in_crd1_receiver,
            in_ref1: in_ref1_receiver,
            in_crd2: in_crd2_receiver,
            in_ref2: in_ref2_receiver,
            out_crd: out_crd_sender,
            out_ref1: out_ref1_sender,
            out_ref2: out_ref2_sender,
        };
        let mut intersect = Union::new(data);
        let mut gen1 = GeneratorContext::new(in_crd1, in_crd1_sender);
        let mut gen2 = GeneratorContext::new(in_ref1, in_ref1_sender);
        let mut gen3 = GeneratorContext::new(in_crd2, in_crd2_sender);
        let mut gen4 = GeneratorContext::new(in_ref2, in_ref2_sender);
        let mut crd_checker = CheckerContext::new(out_crd, out_crd_receiver);
        let mut ref1_checker = CheckerContext::new(out_ref1, out_ref1_receiver);
        let mut ref2_checker = CheckerContext::new(out_ref2, out_ref2_receiver);
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut gen2);
        parent.add_child(&mut gen3);
        parent.add_child(&mut gen4);
        parent.add_child(&mut crd_checker);
        parent.add_child(&mut ref1_checker);
        parent.add_child(&mut ref2_checker);
        parent.add_child(&mut intersect);
        parent.init();
        parent.run();
        parent.cleanup();
    }
}
