use crate::{
    channel::{
        utils::{dequeue, enqueue},
        ChannelElement, Receiver, Sender,
    },
    context::{view::TimeManager, Context},
    types::{Cleanable, DAMType},
};

use super::primitive::Token;

pub struct ReduceData<ValType, StopType> {
    // curr_ref: Token,
    // curr_crd: Stream,
    in_val: Receiver<Token<ValType, StopType>>,
    out_val: Sender<Token<ValType, StopType>>,
    // out_crd: Sender<Token<ValType, StopType>>,
    // end_fiber: bool,
    // emit_tkn: bool,
    // meta_dim: i32,
    // start_addr: i32,
    // end_addr: i32,
    // begin: bool,
}

impl<ValType: DAMType, StopType: DAMType> Cleanable for ReduceData<ValType, StopType> {
    fn cleanup(&mut self) {
        self.in_val.cleanup();
        self.out_val.cleanup();
    }
}

pub struct Reduce<ValType, StopType> {
    reduce_data: ReduceData<ValType, StopType>,
    // meta_dim: ValType,
    time: TimeManager,
}

impl<ValType: DAMType, StopType: DAMType> Reduce<ValType, StopType>
where
    Reduce<ValType, StopType>: Context,
{
    pub fn new(reduce_data: ReduceData<ValType, StopType>) -> Self {
        let red = Reduce {
            reduce_data,
            time: TimeManager::default(),
        };
        (red.reduce_data.in_val).attach_receiver(&red);
        (red.reduce_data.out_val).attach_sender(&red);

        red
    }
}

impl<ValType, StopType> Context for Reduce<ValType, StopType>
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

    fn cleanup(&mut self) {
        self.reduce_data.cleanup();
        self.time.cleanup();
    }

    fn view(&self) -> Box<dyn crate::context::ContextView> {
        Box::new(self.time.view())
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
    };

    use super::Reduce;
    use super::ReduceData;

    #[test]
    fn reduce_2d_test() {
        let in_val = || {
            vec![5u32, 5]
                .into_iter()
                .map(Token::Val)
                .chain([Token::Stop(0u32)])
                .chain(vec![5].into_iter().map(Token::Val))
                .chain([Token::Stop(0u32)])
                .chain(vec![4, 8].into_iter().map(Token::Val))
                .chain([Token::Stop(0u32)])
                .chain(vec![4, 3].into_iter().map(Token::Val))
                .chain([Token::Stop(0u32)])
                .chain(vec![4, 3].into_iter().map(Token::Val))
                .chain([Token::Stop(1), Token::Done])
        };
        let out_val = || {
            vec![10u32, 5, 12, 7, 7]
                .into_iter()
                .map(Token::Val)
                .chain([Token::Stop(0u32), Token::Done])
        };
        reduce_test(in_val, out_val);
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
}
