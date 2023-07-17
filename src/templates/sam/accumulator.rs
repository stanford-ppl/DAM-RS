use core::hash::Hash;
use std::collections::BTreeMap;

use crate::{channel::utils::peek_next, context::Context};
use dam_core::identifier::Identifier;
use dam_core::TimeManager;
use dam_macros::{cleanup, identifiable, time_managed};

use crate::{
    channel::{
        utils::{dequeue, enqueue},
        ChannelElement, Receiver, Sender,
    },
    types::{Cleanable, DAMType},
};

use super::primitive::Token;

pub struct ReduceData<ValType, StopType> {
    pub in_val: Receiver<Token<ValType, StopType>>,
    pub out_val: Sender<Token<ValType, StopType>>,
}

impl<ValType: DAMType, StopType: DAMType> Cleanable for ReduceData<ValType, StopType> {
    fn cleanup(&mut self) {
        self.in_val.cleanup();
        self.out_val.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct Reduce<ValType, StopType> {
    reduce_data: ReduceData<ValType, StopType>,
}

impl<ValType: DAMType, StopType: DAMType> Reduce<ValType, StopType>
where
    Reduce<ValType, StopType>: Context,
{
    pub fn new(reduce_data: ReduceData<ValType, StopType>) -> Self {
        let red = Reduce {
            reduce_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (red.reduce_data.in_val).attach_receiver(&red);
        (red.reduce_data.out_val).attach_sender(&red);

        red
    }
}

impl<ValType, StopType> Context for Reduce<ValType, StopType>
where
    ValType: DAMType
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

    fn run(&mut self) {
        let mut sum = ValType::default();
        loop {
            match dequeue(&mut self.time, &mut self.reduce_data.in_val) {
                Ok(curr_in) => match curr_in.data {
                    Token::Val(val) => {
                        sum += val;
                    }
                    Token::Stop(stkn) => {
                        let curr_time = self.time.tick();
                        enqueue(
                            &mut self.time,
                            &mut self.reduce_data.out_val,
                            ChannelElement::new(curr_time + 1, Token::Val(sum)),
                        )
                        .unwrap();
                        sum = ValType::default();
                        if stkn != StopType::default() {
                            enqueue(
                                &mut self.time,
                                &mut self.reduce_data.out_val,
                                ChannelElement::new(curr_time + 1, Token::Stop(stkn - 1)),
                            )
                            .unwrap();
                        }
                    }
                    Token::Empty => {
                        continue;
                    }
                    Token::Done => {
                        let curr_time = self.time.tick();
                        enqueue(
                            &mut self.time,
                            &mut self.reduce_data.out_val,
                            ChannelElement::new(curr_time + 1, Token::Done),
                        )
                        .unwrap();
                        return;
                    }
                },
                Err(_) => {
                    panic!("Unexpected end of stream");
                }
            }
            // println!("seg: {:?}", self.seg_arr);
            // println!("crd: {:?}", self.crd_arr);
            self.time.incr_cycles(1);
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.reduce_data.cleanup();
    }
}

pub struct Spacc1Data<CrdType, ValType, StopType> {
    pub in_val: Receiver<Token<ValType, StopType>>,
    pub in_crd_outer: Receiver<Token<CrdType, StopType>>,
    pub in_crd_inner: Receiver<Token<CrdType, StopType>>,
    pub out_val: Sender<Token<ValType, StopType>>,
    pub out_crd_inner: Sender<Token<CrdType, StopType>>,
}

impl<CrdType: DAMType, ValType: DAMType, StopType: DAMType> Cleanable
    for Spacc1Data<CrdType, ValType, StopType>
{
    fn cleanup(&mut self) {
        self.in_val.cleanup();
        self.in_crd_outer.cleanup();
        self.in_crd_inner.cleanup();
        self.out_val.cleanup();
        self.out_crd_inner.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct Spacc1<CrdType, ValType, StopType> {
    spacc1_data: Spacc1Data<CrdType, ValType, StopType>,
}

impl<CrdType: DAMType, ValType: DAMType, StopType: DAMType> Spacc1<CrdType, ValType, StopType>
where
    Spacc1<CrdType, ValType, StopType>: Context,
{
    pub fn new(spacc1_data: Spacc1Data<CrdType, ValType, StopType>) -> Self {
        let red = Spacc1 {
            spacc1_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (red.spacc1_data.in_crd_outer).attach_receiver(&red);
        (red.spacc1_data.in_crd_inner).attach_receiver(&red);
        (red.spacc1_data.in_val).attach_receiver(&red);
        (red.spacc1_data.out_crd_inner).attach_sender(&red);
        (red.spacc1_data.out_val).attach_sender(&red);

        red
    }
}

impl<CrdType, ValType, StopType> Context for Spacc1<CrdType, ValType, StopType>
where
    CrdType: DAMType + Hash + std::cmp::Eq + std::cmp::PartialEq + std::cmp::Ord,
    ValType: DAMType
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

    fn run(&mut self) {
        let mut accum_storage: BTreeMap<CrdType, ValType> = BTreeMap::new();

        loop {
            let in_ocrd = peek_next(&mut self.time, &mut self.spacc1_data.in_crd_outer);
            let in_icrd = peek_next(&mut self.time, &mut self.spacc1_data.in_crd_inner);
            let in_val = peek_next(&mut self.time, &mut self.spacc1_data.in_val);

            // dbg!(in_ocrd.as_ref().unwrap().data.clone());
            // dbg!(in_icrd.as_ref().unwrap().data.clone());
            // dbg!(in_val.as_ref().unwrap().data.clone());

            match in_icrd {
                Ok(curr_in) => {
                    // let val_deq = dequeue(&mut self.time, &mut self.spacc1_data.in_val);
                    match in_ocrd.unwrap().data.clone() {
                        Token::Val(_) => {
                            // In acc state
                            match in_val.unwrap().data.clone() {
                                Token::Val(val) => match curr_in.data.clone() {
                                    Token::Val(crd) => {
                                        *accum_storage.entry(crd).or_insert(ValType::default()) +=
                                            val.clone();
                                        // dbg!(accum_storage.clone());
                                        dequeue(&mut self.time, &mut self.spacc1_data.in_crd_inner)
                                            .unwrap();
                                        dequeue(&mut self.time, &mut self.spacc1_data.in_val)
                                            .unwrap();
                                    }
                                    Token::Stop(_) => {
                                        dequeue(&mut self.time, &mut self.spacc1_data.in_crd_outer)
                                            .unwrap();
                                    }
                                    Token::Done => {
                                        panic!("Shouldn't have a done token for coords if in acc state");
                                    }
                                    _ => {
                                        panic!("Invalid token");
                                    }
                                },
                                Token::Stop(_) => {
                                    dequeue(&mut self.time, &mut self.spacc1_data.in_crd_outer)
                                        .unwrap();
                                    dequeue(&mut self.time, &mut self.spacc1_data.in_crd_inner)
                                        .unwrap();
                                    dequeue(&mut self.time, &mut self.spacc1_data.in_val).unwrap();
                                    continue;
                                }
                                _ => {
                                    panic!("Invalid token found, val token should be similar type to crd");
                                }
                            }
                        }
                        Token::Stop(stkn) => {
                            for (key, value) in &accum_storage {
                                let icrd_chan_elem = ChannelElement::new(
                                    self.time.tick() + 1,
                                    // Token::Val(accum_storage.keys().next().unwrap().clone()),
                                    Token::Val(key.clone()),
                                );
                                enqueue(
                                    &mut self.time,
                                    &mut self.spacc1_data.out_crd_inner,
                                    icrd_chan_elem,
                                )
                                .unwrap();
                                let val_chan_elem = ChannelElement::new(
                                    self.time.tick() + 1,
                                    Token::<ValType, StopType>::Val(value.clone()),
                                );
                                enqueue(
                                    &mut self.time,
                                    &mut self.spacc1_data.out_val,
                                    val_chan_elem,
                                )
                                .unwrap();
                                // dbg!(key.clone());
                                // dbg!(value.clone());
                            }
                            let val_stkn_chan_elem = ChannelElement::new(
                                self.time.tick() + 1,
                                Token::Stop(stkn.clone()),
                            );
                            enqueue(
                                &mut self.time,
                                &mut self.spacc1_data.out_val,
                                val_stkn_chan_elem.clone(),
                            )
                            .unwrap();
                            let crd_stkn_chan_elem = ChannelElement::new(
                                self.time.tick() + 1,
                                Token::Stop(stkn.clone()),
                            );
                            enqueue(
                                &mut self.time,
                                &mut self.spacc1_data.out_crd_inner,
                                crd_stkn_chan_elem,
                            )
                            .unwrap();
                            accum_storage.clear();
                            dequeue(&mut self.time, &mut self.spacc1_data.in_crd_outer).unwrap();
                        }
                        Token::Done => {
                            let icrd_chan_elem =
                                ChannelElement::new(self.time.tick() + 1, Token::Done);
                            enqueue(
                                &mut self.time,
                                &mut self.spacc1_data.out_crd_inner,
                                icrd_chan_elem,
                            )
                            .unwrap();
                            let val_chan_elem =
                                ChannelElement::new(self.time.tick() + 1, Token::Done);
                            enqueue(&mut self.time, &mut self.spacc1_data.out_val, val_chan_elem)
                                .unwrap();
                            return;
                        }
                        _ => {
                            panic!("Invalid case in spacc1");
                        }
                    }
                }
                Err(_) => {
                    panic!("Unexpected error in dequeueing of crd_outer");
                }
            }
            self.time.incr_cycles(1);
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.spacc1_data.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct MaxReduce<ValType, StopType> {
    max_reduce_data: ReduceData<ValType, StopType>,
    min_val: ValType,
}

impl<ValType: DAMType, StopType: DAMType> MaxReduce<ValType, StopType>
where
    MaxReduce<ValType, StopType>: Context,
{
    pub fn new(max_reduce_data: ReduceData<ValType, StopType>, min_val: ValType) -> Self {
        let red = MaxReduce {
            max_reduce_data,
            min_val,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (red.max_reduce_data.in_val).attach_receiver(&red);
        (red.max_reduce_data.out_val).attach_sender(&red);

        red
    }
}

impl<ValType, StopType> Context for MaxReduce<ValType, StopType>
where
    ValType: DAMType
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

    fn run(&mut self) {
        let mut max_elem = self.min_val.clone();
        loop {
            match dequeue(&mut self.time, &mut self.max_reduce_data.in_val) {
                Ok(curr_in) => match curr_in.data {
                    Token::Val(val) => {
                        // max_elem = max(val, max_elem);
                        match val.lt(&max_elem) {
                            true => (),
                            false => max_elem = val,
                        }
                    }
                    Token::Stop(stkn) => {
                        let curr_time = self.time.tick();
                        enqueue(
                            &mut self.time,
                            &mut self.max_reduce_data.out_val,
                            ChannelElement::new(curr_time + 1, Token::Val(max_elem)),
                        )
                        .unwrap();
                        max_elem = ValType::default();
                        if stkn != StopType::default() {
                            enqueue(
                                &mut self.time,
                                &mut self.max_reduce_data.out_val,
                                ChannelElement::new(curr_time + 1, Token::Stop(stkn - 1)),
                            )
                            .unwrap();
                        }
                    }
                    Token::Empty => {
                        continue;
                    }
                    Token::Done => {
                        let curr_time = self.time.tick();
                        enqueue(
                            &mut self.time,
                            &mut self.max_reduce_data.out_val,
                            ChannelElement::new(curr_time + 1, Token::Done),
                        )
                        .unwrap();
                        return;
                    }
                },
                Err(_) => {
                    panic!("Unexpected end of stream");
                }
            }
            // println!("seg: {:?}", self.seg_arr);
            // println!("crd: {:?}", self.crd_arr);
            self.time.incr_cycles(1);
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.max_reduce_data.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use dam_core::TimeViewable;

    use crate::{
        channel::unbounded,
        context::{
            checker_context::CheckerContext, generator_context::GeneratorContext,
            parent::BasicParentContext, Context, ParentContext,
        },
        templates::sam::primitive::Token,
        token_vec,
    };

    use super::{MaxReduce, Reduce, Spacc1};
    use super::{ReduceData, Spacc1Data};

    #[test]
    fn reduce_2d_test() {
        let in_val = || {
            token_vec!(u32; u32; 5, 5, "S0", 5, "S0", 4, 8, "S0", 4, 3, "S0", 4, 3, "S1", "D")
                .into_iter()
        };
        let out_val = || token_vec!(u32; u32; 10, 5, 12, 7, 7, "S0", "D").into_iter();
        reduce_test(in_val, out_val);
    }

    #[test]
    fn spacc1_2d_test() {
        let in_ocrd = || token_vec!(u32; u32; 0, 2, "S0", 2, "S1", "D").into_iter();
        let in_icrd =
            || token_vec!(u32; u32; 0, 2, 3, "S0", 0, 2, 3, "S1", 0, 2, 3, "S2", "D").into_iter();
        let in_val = || {
            token_vec!(f32; u32; 50.0, 5.0, 10.0, "S0", 40.0, 4.0, 8.0, "S1", -40.0, 33.0, 36.0, "S2", "D")
                    .into_iter()
        };
        let out_icrd = || token_vec!(u32; u32; 0, 2, 3, "S0", 0, 2, 3, "S1", "D").into_iter();
        let out_val = || {
            token_vec!(f32; u32; 90.0, 9.0, 18.0, "S0", -40.0, 33.0, 36.0, "S1", "D").into_iter()
        };
        spacc1_test(in_ocrd, in_icrd, in_val, out_icrd, out_val);
    }

    #[test]
    fn max_reduce_2d_test() {
        let in_val = || {
            token_vec!(f32; u32; 5.0, 5.0, "S0", 5.0, "S0", 4.0, 8.0, "S0", 4.0, 3.0, "S0", 4.0, 3.0, "S1", "D")
                .into_iter()
        };
        let out_val = || token_vec!(f32; u32; 5.0, 5.0, 8.0, 4.0, 4.0, "S0", "D").into_iter();
        max_reduce_test(in_val, out_val);
    }

    fn reduce_test<IRT, ORT>(in_val: fn() -> IRT, out_val: fn() -> ORT)
    where
        IRT: Iterator<Item = Token<u32, u32>> + 'static,
        ORT: Iterator<Item = Token<u32, u32>> + 'static,
    {
        let (in_val_sender, in_val_receiver) = unbounded::<Token<u32, u32>>();
        let (out_val_sender, out_val_receiver) = unbounded::<Token<u32, u32>>();
        let data = ReduceData::<u32, u32> {
            in_val: in_val_receiver,
            out_val: out_val_sender,
        };
        let mut red = Reduce::new(data);
        let mut gen1 = GeneratorContext::new(in_val, in_val_sender);
        let mut val_checker = CheckerContext::new(out_val, out_val_receiver);
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut val_checker);
        parent.add_child(&mut red);
        parent.init();
        parent.run();
        parent.cleanup();
    }

    fn spacc1_test<IRT1, IRT2, IRT3, ORT1, ORT2>(
        in_ocrd: fn() -> IRT1,
        in_icrd: fn() -> IRT2,
        in_val: fn() -> IRT3,
        out_icrd: fn() -> ORT1,
        out_val: fn() -> ORT2,
    ) where
        IRT1: Iterator<Item = Token<u32, u32>> + 'static,
        IRT2: Iterator<Item = Token<u32, u32>> + 'static,
        IRT3: Iterator<Item = Token<f32, u32>> + 'static,
        ORT1: Iterator<Item = Token<u32, u32>> + 'static,
        ORT2: Iterator<Item = Token<f32, u32>> + 'static,
    {
        let (in_ocrd_sender, in_ocrd_receiver) = unbounded::<Token<u32, u32>>();
        let (in_icrd_sender, in_icrd_receiver) = unbounded::<Token<u32, u32>>();
        let (in_val_sender, in_val_receiver) = unbounded::<Token<f32, u32>>();
        let (out_val_sender, out_val_receiver) = unbounded::<Token<f32, u32>>();
        let (out_icrd_sender, out_icrd_receiver) = unbounded::<Token<u32, u32>>();
        let data = Spacc1Data::<u32, f32, u32> {
            in_crd_outer: in_ocrd_receiver,
            in_crd_inner: in_icrd_receiver,
            in_val: in_val_receiver,
            out_val: out_val_sender,
            out_crd_inner: out_icrd_sender,
        };
        let mut red = Spacc1::new(data);
        let mut gen1 = GeneratorContext::new(in_ocrd, in_ocrd_sender);
        let mut gen2 = GeneratorContext::new(in_icrd, in_icrd_sender);
        let mut gen3 = GeneratorContext::new(in_val, in_val_sender);
        let mut icrd_checker = CheckerContext::new(out_icrd, out_icrd_receiver);
        let mut val_checker = CheckerContext::new(out_val, out_val_receiver);
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut gen2);
        parent.add_child(&mut gen3);
        parent.add_child(&mut icrd_checker);
        parent.add_child(&mut val_checker);
        parent.add_child(&mut red);
        parent.init();
        parent.run();
        parent.cleanup();
    }

    fn max_reduce_test<IRT, ORT>(in_val: fn() -> IRT, out_val: fn() -> ORT)
    where
        IRT: Iterator<Item = Token<f32, u32>> + 'static,
        ORT: Iterator<Item = Token<f32, u32>> + 'static,
    {
        let (in_val_sender, in_val_receiver) = unbounded::<Token<f32, u32>>();
        let (out_val_sender, out_val_receiver) = unbounded::<Token<f32, u32>>();
        let data = ReduceData::<f32, u32> {
            in_val: in_val_receiver,
            out_val: out_val_sender,
        };
        let mut red = MaxReduce::new(data, f32::MIN);
        let mut gen1 = GeneratorContext::new(in_val, in_val_sender);
        let mut val_checker = CheckerContext::new(out_val, out_val_receiver);
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut val_checker);
        parent.add_child(&mut red);
        parent.init();
        parent.run();
        parent.cleanup();
    }
}
