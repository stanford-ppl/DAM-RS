use crate::{
    channel::{
        utils::{dequeue, enqueue},
        ChannelElement, Receiver, Sender,
    },
    context::{view::TimeManager, Context},
    types::{Cleanable, DAMType},
};

use super::primitive::Token;

pub struct ArrayData<ValType, StopType> {
    in_ref: Receiver<Token<ValType, StopType>>,
    out_val: Sender<Token<ValType, StopType>>,
}

impl<ValType: DAMType, StopType: DAMType> Cleanable for ArrayData<ValType, StopType> {
    fn cleanup(&mut self) {
        self.in_ref.cleanup();
        self.out_val.cleanup();
    }
}

pub struct Array<ValType, StopType> {
    array_data: ArrayData<ValType, StopType>,
    val_arr: Vec<ValType>,
    time: TimeManager,
}

impl<ValType: DAMType, StopType: DAMType> Array<ValType, StopType>
where
    Array<ValType, StopType>: Context,
{
    pub fn new(array_data: ArrayData<ValType, StopType>, val_arr: Vec<ValType>) -> Self {
        let arr = Array {
            array_data,
            val_arr,
            time: TimeManager::default(),
        };
        (arr.array_data.in_ref).attach_receiver(&arr);
        (arr.array_data.out_val).attach_sender(&arr);

        arr
    }
}

impl<ValType, StopType> Context for Array<ValType, StopType>
where
    ValType: DAMType
        + std::ops::AddAssign<u32>
        + std::ops::Mul<ValType, Output = ValType>
        + std::ops::Add<ValType, Output = ValType>
        + std::cmp::PartialOrd<ValType>,
    ValType: TryInto<usize>,
    <ValType as TryInto<usize>>::Error: std::fmt::Debug,
    StopType: DAMType + std::ops::Add<u32, Output = StopType>,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            match dequeue(&mut self.time, &mut self.array_data.in_ref) {
                Ok(curr_in) => match curr_in.data {
                    Token::Val(val) => {
                        let idx: usize = val.try_into().unwrap();
                        let channel_elem = ChannelElement::new(
                            self.time.tick() + 1,
                            Token::Val(self.val_arr[idx]),
                        );
                        enqueue(&mut self.time, &mut self.array_data.out_val, channel_elem)
                            .unwrap();
                    }
                    Token::Stop(stkn) => {
                        let channel_elem =
                            ChannelElement::new(self.time.tick() + 1, Token::Stop(stkn));
                        enqueue(&mut self.time, &mut self.array_data.out_val, channel_elem)
                            .unwrap();
                    }
                    Token::Empty => {
                        let channel_elem = ChannelElement::new(
                            self.time.tick() + 1,
                            Token::Val(ValType::default()),
                        );
                        enqueue(&mut self.time, &mut self.array_data.out_val, channel_elem)
                            .unwrap();
                    }
                    Token::Done => {
                        let channel_elem = ChannelElement::new(self.time.tick() + 1, Token::Done);
                        enqueue(&mut self.time, &mut self.array_data.out_val, channel_elem)
                            .unwrap();
                        return;
                    }
                },
                Err(_) => {
                    panic!("Unexpected end of stream");
                }
            }
            self.time.incr_cycles(1);
        }
    }

    fn cleanup(&mut self) {
        self.array_data.cleanup();
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
        token_vec,
    };

    use super::Array;
    use super::ArrayData;

    #[test]
    fn array_2d_test() {
        let in_ref = || {
            token_vec![u32; u32; "N", 0, 1, 2, "S0", "N", "N", "S0", 2, 3, 4, "S0", "N", "N", "S1", "D"].into_iter()
        };
        let out_val = || {
            token_vec!(u32; u32; 0, 1, 2, 3, "S0", 0, 0, "S0", 3, 4, 5, "S0", 0, 0, "S1", "D")
                .into_iter()
        };
        let val_arr = vec![1u32, 2, 3, 4, 5];
        array_test(in_ref, out_val, val_arr);
    }

    fn array_test<IRT, ORT>(in_ref: fn() -> IRT, out_val: fn() -> ORT, val_arr: Vec<u32>)
    where
        IRT: Iterator<Item = Token<u32, u32>> + 'static,
        ORT: Iterator<Item = Token<u32, u32>> + 'static,
    {
        let (in_ref_sender, in_ref_receiver) = unbounded::<Token<u32, u32>>();
        let (out_val_sender, out_val_receiver) = unbounded::<Token<u32, u32>>();
        let data = ArrayData::<u32, u32> {
            in_ref: in_ref_receiver,
            out_val: out_val_sender,
        };
        let mut arr = Array::new(data, val_arr);
        let mut gen1 = GeneratorContext::new(in_ref, in_ref_sender);
        let mut out_val_checker = CheckerContext::new(out_val, out_val_receiver);
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut out_val_checker);
        parent.add_child(&mut arr);
        parent.init();
        parent.run();
        parent.cleanup();
    }
}
