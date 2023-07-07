use crate::{
    channel::{utils::dequeue, Receiver},
    context::{view::TimeManager, Context},
    types::{Cleanable, DAMType},
};

use super::primitive::Token;

pub struct WrScanData<ValType, StopType> {
    pub input: Receiver<Token<ValType, StopType>>,
}

impl<ValType: DAMType, StopType: DAMType> Cleanable for WrScanData<ValType, StopType> {
    fn cleanup(&mut self) {
        self.input.cleanup();
    }
}

pub struct CompressedWrScan<ValType, StopType> {
    wr_scan_data: WrScanData<ValType, StopType>,
    // meta_dim: ValType,
    pub seg_arr: Vec<ValType>,
    pub crd_arr: Vec<ValType>,
    time: TimeManager,
}

impl<ValType: DAMType, StopType: DAMType> CompressedWrScan<ValType, StopType>
where
    CompressedWrScan<ValType, StopType>: Context,
{
    pub fn new(
        wr_scan_data: WrScanData<ValType, StopType>,
        seg_arr: Vec<ValType>,
        crd_arr: Vec<ValType>,
    ) -> Self {
        let cwr = CompressedWrScan {
            wr_scan_data,
            seg_arr,
            crd_arr,
            time: TimeManager::default(),
        };
        (cwr.wr_scan_data.input).attach_receiver(&cwr);

        cwr
    }
}

impl<ValType, StopType> Context for CompressedWrScan<ValType, StopType>
where
    ValType: DAMType
        + std::ops::AddAssign<u32>
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
            match dequeue(&mut self.time, &mut self.wr_scan_data.input) {
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

    fn cleanup(&mut self) {
        self.wr_scan_data.cleanup();
        self.time.cleanup();
    }

    fn view(&self) -> Box<dyn crate::context::ContextView> {
        Box::new(self.time.view())
    }
}

pub struct ValsWrScan<ValType, StopType> {
    vals_data: WrScanData<ValType, StopType>,
    // meta_dim: ValType,
    pub out_val: Vec<ValType>,
    time: TimeManager,
}

impl<ValType: DAMType, StopType: DAMType> ValsWrScan<ValType, StopType>
where
    ValsWrScan<ValType, StopType>: Context,
{
    pub fn new(vals_data: WrScanData<ValType, StopType>, out_val: Vec<ValType>) -> Self {
        let vals = ValsWrScan {
            vals_data,
            out_val,
            time: TimeManager::default(),
        };
        (vals.vals_data.input).attach_receiver(&vals);

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
            match dequeue(&mut self.time, &mut self.vals_data.input) {
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

    fn cleanup(&mut self) {
        self.vals_data.cleanup();
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
            generator_context::GeneratorContext, parent::BasicParentContext, Context, ParentContext,
        },
        templates::sam::primitive::Token,
    };

    use super::CompressedWrScan;
    use super::WrScanData;

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
        gold_seg_arr: Vec<u32>,
        gold_crd_arr: Vec<u32>,
    ) where
        IRT: Iterator<Item = Token<u32, u32>> + 'static,
    {
        let (input_sender, input_receiver) = unbounded::<Token<u32, u32>>();
        let data = WrScanData::<u32, u32> {
            input: input_receiver,
        };
        let seg_arr: Vec<u32> = Vec::new();
        let crd_arr: Vec<u32> = Vec::new();
        let mut cr = CompressedWrScan::new(data, seg_arr, crd_arr);
        let mut gen1 = GeneratorContext::new(input, input_sender);
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut cr);
        parent.init();
        parent.run();
        parent.cleanup();
        assert_eq!(vec_compare(&gold_seg_arr, &cr.seg_arr), true);
        assert_eq!(vec_compare(&gold_crd_arr, &cr.crd_arr), true);
    }
}
