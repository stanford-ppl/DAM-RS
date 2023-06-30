use crate::{
    channel::{utils::dequeue, Receiver},
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

        red
    }
}

impl<ValType, StopType> Context for Reduce<ValType, StopType>
where
    ValType: DAMType
        + std::ops::AddAssign<u32>
        + std::ops::Mul<ValType, Output = ValType>
        + std::ops::Add<ValType, Output = ValType>
        + std::cmp::PartialOrd<ValType>,
    StopType: DAMType + std::ops::Add<u32, Output = StopType>,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        // let mut curr_crd: Token<ValType, StopType>
        let mut emit_stkn = false;
        let sum = ValType::default();
        loop {
            match dequeue(&mut self.time, &mut self.reduce_data.in_val) {
                Ok(curr_in) => match curr_in.data {
                    Token::Val(val) => {
                        sum += val;
                    }
                    Token::Stop(_) if !end_fiber => {
                        self.seg_arr.push(curr_crd_cnt);
                        end_fiber = true;
                    }
                    Token::Empty | Token::Stop(_) => {
                        // TODO: Maybe needs to be processed too
                        // panic!("Reached panic in wr scanner");
                        continue;
                    }
                    Token::Done => return,
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
