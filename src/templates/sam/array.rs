use dam_core::{identifier::Identifier, TimeManager};
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

pub struct ArrayData<RefType: Clone, ValType: Clone, StopType: Clone> {
    pub in_ref: Receiver<Token<RefType, StopType>>,
    pub out_val: Sender<Token<ValType, StopType>>,
}

impl<RefType: DAMType, ValType: DAMType, StopType: DAMType> Cleanable
    for ArrayData<RefType, ValType, StopType>
{
    fn cleanup(&mut self) {
        self.in_ref.cleanup();
        self.out_val.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct Array<RefType: Clone, ValType: Clone, StopType: Clone> {
    array_data: ArrayData<RefType, ValType, StopType>,
    val_arr: Vec<ValType>,
}

impl<RefType: DAMType, ValType: DAMType, StopType: DAMType> Array<RefType, ValType, StopType>
where
    Array<RefType, ValType, StopType>: Context,
{
    pub fn new(array_data: ArrayData<RefType, ValType, StopType>, val_arr: Vec<ValType>) -> Self {
        let arr = Array {
            array_data,
            val_arr,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (arr.array_data.in_ref).attach_receiver(&arr);
        (arr.array_data.out_val).attach_sender(&arr);

        arr
    }
}

impl<RefType, ValType, StopType> Context for Array<RefType, ValType, StopType>
where
    RefType: DAMType
        + std::ops::Mul<RefType, Output = RefType>
        + std::ops::Add<RefType, Output = RefType>,
    RefType: TryInto<usize>,
    <RefType as TryInto<usize>>::Error: std::fmt::Debug,
    ValType: DAMType,
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
                            Token::Val(self.val_arr[idx].clone()),
                        );
                        enqueue(&mut self.time, &mut self.array_data.out_val, channel_elem)
                            .unwrap();
                        // dbg!(self.val_arr[idx].clone());
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

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.array_data.cleanup();
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
        let mut parent = Program::default();
        let (in_ref_sender, in_ref_receiver) = parent.unbounded::<Token<u32, u32>>();
        let (out_val_sender, out_val_receiver) = parent.unbounded::<Token<u32, u32>>();
        let data = ArrayData::<u32, u32, u32> {
            in_ref: in_ref_receiver,
            out_val: out_val_sender,
        };
        let arr = Array::new(data, val_arr);
        let gen1 = GeneratorContext::new(in_ref, in_ref_sender);
        let out_val_checker = CheckerContext::new(out_val, out_val_receiver);
        parent.add_child(gen1);
        parent.add_child(out_val_checker);
        parent.add_child(arr);
        parent.init();
        parent.run();
    }
}
