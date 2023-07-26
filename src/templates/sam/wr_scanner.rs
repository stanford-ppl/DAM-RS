use dam_core::{identifier::Identifier, TimeManager};
use dam_macros::{cleanup, identifiable, time_managed};

use crate::{
    channel::{utils::dequeue, Receiver},
    context::Context,
    types::{Cleanable, DAMType},
};

use super::primitive::Token;

// pub struct WrScanData<ValType: Clone, StopType: Clone> {
//     pub input: Receiver<Token<ValType, StopType>>,
// }

// impl<ValType: DAMType, StopType: DAMType> Cleanable for WrScanData<ValType, StopType> {
//     fn cleanup(&mut self) {
//         self.input.cleanup();
//     }
// }

#[time_managed]
#[identifiable]
pub struct CompressedWrScan<ValType: Clone, StopType: Clone> {
    // wr_scan_data: WrScanData<ValType, StopType>,
    pub input: Receiver<Token<ValType, StopType>>,
    pub seg_arr: Vec<ValType>,
    pub crd_arr: Vec<ValType>,
}

impl<ValType: DAMType, StopType: DAMType> CompressedWrScan<ValType, StopType>
where
    CompressedWrScan<ValType, StopType>: Context,
{
    pub fn new(input: Receiver<Token<ValType, StopType>>) -> Self {
        let cwr = CompressedWrScan {
            input,
            seg_arr: vec![],
            crd_arr: vec![],
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (cwr).input.attach_receiver(&cwr);

        cwr
    }
}

impl<ValType, StopType> Context for CompressedWrScan<ValType, StopType>
where
    ValType: DAMType
        + std::ops::AddAssign<u32>
        + std::ops::AddAssign<ValType>
        + std::ops::Mul<ValType, Output = ValType>
        + std::ops::Add<ValType, Output = ValType>
        + std::cmp::PartialOrd<ValType>,
    StopType: DAMType + std::ops::Add<u32, Output = StopType>,
{
    fn init(&mut self) {
        // default is 0
        self.seg_arr.push(ValType::default());
    }

    fn run(&mut self) {
        // let mut curr_crd: Token<ValType, StopType>
        let mut curr_crd_cnt: ValType = ValType::default();
        let mut end_fiber = false;
        loop {
            match dequeue(&mut self.time, &mut self.input) {
                Ok(curr_in) => match curr_in.data {
                    Token::Val(val) => {
                        self.crd_arr.push(val);
                        curr_crd_cnt += 1;
                        end_fiber = false;
                    }
                    Token::Stop(_) if !end_fiber => {
                        self.seg_arr.push(curr_crd_cnt.clone());
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

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.input.cleanup();
        self.time.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct ValsWrScan<ValType: Clone, StopType: Clone> {
    pub input: Receiver<Token<ValType, StopType>>,
    pub out_val: Vec<ValType>,
}

impl<ValType: DAMType, StopType: DAMType> ValsWrScan<ValType, StopType>
where
    ValsWrScan<ValType, StopType>: Context,
{
    pub fn new(input: Receiver<Token<ValType, StopType>>) -> Self {
        let vals = ValsWrScan {
            input,
            out_val: vec![],
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (vals.input).attach_receiver(&vals);

        vals
    }
}

impl<ValType, StopType> Context for ValsWrScan<ValType, StopType>
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
            match dequeue(&mut self.time, &mut self.input) {
                Ok(curr_in) => match curr_in.data {
                    Token::Val(val) => {
                        self.out_val.push(val);
                    }
                    Token::Empty | Token::Stop(_) => {
                        continue;
                    }
                    Token::Done => return,
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
        self.input.cleanup();
        self.time.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        context::generator_context::GeneratorContext, simulation::Program,
        templates::sam::primitive::Token,
    };

    use super::CompressedWrScan;

    fn vec_compare(va: &[u32], vb: &[u32]) -> bool {
        (va.len() == vb.len()) &&  // zip stops at the shortest
        va.iter()
        .zip(vb)
        .all(|(a,b)| *a == *b)
    }

    #[test]
    fn cwr_1d_test() {
        let gold_seg_arr = vec![0u32, 3, 6];
        let gold_crd_arr = vec![0u32, 2, 3, 0, 2, 3];
        let input = || {
            vec![0u32, 2, 3]
                .into_iter()
                .map(Token::Val)
                .chain([Token::Stop(0u32)])
                .chain(vec![0, 2, 3].into_iter().map(Token::Val))
                .chain([Token::Stop(1), Token::Done])
        };
        compressed_wr_scan_test(input, gold_seg_arr, gold_crd_arr);
    }

    fn compressed_wr_scan_test<IRT>(
        input: fn() -> IRT,
        _gold_seg_arr: Vec<u32>,
        _gold_crd_arr: Vec<u32>,
    ) where
        IRT: Iterator<Item = Token<u32, u32>> + 'static,
    {
        let mut parent = Program::default();
        let (input_sender, input_receiver) = parent.unbounded::<Token<u32, u32>>();
        let cr = CompressedWrScan::new(input_receiver);
        let gen1 = GeneratorContext::new(input, input_sender);
        parent.add_child(gen1);
        parent.add_child(cr);
        parent.init();
        parent.run();
        // assert_eq!(vec_compare(&gold_seg_arr, &cr.seg_arr), true);
        // assert_eq!(vec_compare(&gold_crd_arr, &cr.crd_arr), true);
    }
}
