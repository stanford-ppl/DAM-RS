use crate::context::Context;
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
    use crate::{
        channel::unbounded,
        context::{
            checker_context::CheckerContext, generator_context::GeneratorContext,
            parent::BasicParentContext, Context, ParentContext,
        },
        templates::sam::primitive::Token,
        token_vec,
    };

    use super::ReduceData;
    use super::{MaxReduce, Reduce};

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
