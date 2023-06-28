use crate::{
    channel::{
        utils::{dequeue, enqueue, peek_next},
        ChannelElement, Receiver, Sender,
    },
    context::{view::TimeManager, Context},
    types::{Cleanable, DAMType},
};

use super::primitive::Token;

pub struct RdScanData<ValType, StopType> {
    // curr_ref: Token,
    // curr_crd: Stream,
    in_ref: Receiver<Token<ValType, StopType>>,
    out_ref: Sender<Token<ValType, StopType>>,
    out_crd: Sender<Token<ValType, StopType>>,
    // end_fiber: bool,
    // emit_tkn: bool,
    // meta_dim: i32,
    // start_addr: i32,
    // end_addr: i32,
    // begin: bool,
}

impl<ValType: DAMType, StopType: DAMType> Cleanable for RdScanData<ValType, StopType> {
    fn cleanup(&mut self) {
        self.in_ref.cleanup();
        self.out_ref.cleanup();
        self.out_crd.cleanup();
    }
}

pub trait CrdRdScan<ValType, StopType> {
    // data: RdScanData<ValType, StopType>,
    fn get_data(&self) -> &mut RdScanData<ValType, StopType>;
    // fn out_ref(&self) -> Token<ValType, StopType>;
}

pub struct UncompressedCrdRdScan<ValType, StopType> {
    rd_scan_data: RdScanData<ValType, StopType>,
    meta_dim: ValType,
    time: TimeManager,
}

pub struct CompressedCrdRdScan<ValType, StopType> {
    rd_scan_data: RdScanData<ValType, StopType>,
    // meta_dim: ValType,
    seg_arr: Vec<ValType>,
    crd_arr: Vec<ValType>,
    time: TimeManager,
}

impl<ValType: DAMType, StopType: DAMType> UncompressedCrdRdScan<ValType, StopType>
where
    UncompressedCrdRdScan<ValType, StopType>: Context,
{
    pub fn new(
        rd_scan_data: RdScanData<ValType, StopType>,
        meta_dim: ValType,
    ) -> UncompressedCrdRdScan<ValType, StopType> {
        let ucr = UncompressedCrdRdScan {
            rd_scan_data,
            meta_dim,
            time: TimeManager::default(),
        };
        (ucr.rd_scan_data.in_ref).attach_receiver(&ucr);
        (ucr.rd_scan_data.out_ref).attach_sender(&ucr);
        (ucr.rd_scan_data.out_crd).attach_sender(&ucr);

        ucr
    }
}

impl<ValType: DAMType, StopType: DAMType> CompressedCrdRdScan<ValType, StopType>
where
    CompressedCrdRdScan<ValType, StopType>: Context,
{
    pub fn new(
        rd_scan_data: RdScanData<ValType, StopType>,
        seg_arr: Vec<ValType>,
        crd_arr: Vec<ValType>,
    ) -> Self {
        let ucr = CompressedCrdRdScan {
            rd_scan_data,
            seg_arr,
            crd_arr,
            time: TimeManager::default(),
        };
        (ucr.rd_scan_data.in_ref).attach_receiver(&ucr);
        (ucr.rd_scan_data.out_ref).attach_sender(&ucr);
        (ucr.rd_scan_data.out_crd).attach_sender(&ucr);

        ucr
    }
}

impl<ValType, StopType> Context for UncompressedCrdRdScan<ValType, StopType>
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
        loop {
            match dequeue(&mut self.time, &mut self.rd_scan_data.in_ref) {
                Ok(curr_ref) => match curr_ref.data {
                    Token::Val(val) => {
                        let mut crd_count: ValType = ValType::default();
                        while crd_count < self.meta_dim {
                            let curr_time = self.time.tick();
                            enqueue(
                                &mut self.time,
                                &mut self.rd_scan_data.out_crd,
                                ChannelElement::new(
                                    curr_time + 1,
                                    super::primitive::Token::Val(crd_count),
                                ),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.rd_scan_data.out_ref,
                                ChannelElement::new(
                                    curr_time + 1,
                                    super::primitive::Token::Val(crd_count + val * self.meta_dim),
                                ),
                            )
                            .unwrap();
                            crd_count += 1;
                            self.time.incr_cycles(1);
                        }
                        let next_tkn =
                            peek_next(&mut self.time, &mut self.rd_scan_data.in_ref).unwrap();
                        let output: Token<ValType, StopType> = match next_tkn.data {
                            Token::Val(_) | Token::Done => Token::Stop(StopType::default()),
                            Token::Stop(stop_tkn) => {
                                dequeue(&mut self.time, &mut self.rd_scan_data.in_ref).unwrap();
                                Token::Stop(stop_tkn + 1)
                            }
                            Token::Empty => {
                                panic!("Invalid empty inside peek");
                            }
                        };
                        // dbg!(output);
                        let curr_time = self.time.tick();
                        enqueue(
                            &mut self.time,
                            &mut self.rd_scan_data.out_crd,
                            ChannelElement::new(curr_time + 1, output),
                        )
                        .unwrap();
                        enqueue(
                            &mut self.time,
                            &mut self.rd_scan_data.out_ref,
                            ChannelElement::new(curr_time + 1, output),
                        )
                        .unwrap();
                    }
                    Token::Stop(token) => {
                        let curr_time = self.time.tick();
                        enqueue(
                            &mut self.time,
                            &mut self.rd_scan_data.out_crd,
                            ChannelElement::new(curr_time + 1, Token::Stop(token + 1)),
                        )
                        .unwrap();
                        enqueue(
                            &mut self.time,
                            &mut self.rd_scan_data.out_ref,
                            ChannelElement::new(curr_time + 1, Token::Stop(token + 1)),
                        )
                        .unwrap();
                    }
                    // Could either be a done token or an empty token
                    // In the case of done token, return
                    tkn @ Token::Done | tkn @ Token::Empty => {
                        let channel_elem = ChannelElement::new(self.time.tick() + 1, tkn);
                        enqueue(&mut self.time, &mut self.rd_scan_data.out_crd, channel_elem)
                            .unwrap();
                        enqueue(&mut self.time, &mut self.rd_scan_data.out_ref, channel_elem)
                            .unwrap();
                        if tkn == Token::Done {
                            return;
                        }
                    }
                },
                Err(_) => panic!("Error: rd_scan_data dequeue error"),
            }
            self.time.incr_cycles(1);
        }
    }

    fn cleanup(&mut self) {
        // self.input_channels.iter_mut().for_each(|chan| {
        // chan.lock().unwrap().close();
        // });
        self.rd_scan_data.cleanup();
        self.time.cleanup();
    }

    fn view(&self) -> Box<dyn crate::context::ContextView> {
        Box::new(self.time.view())
    }
}

impl<ValType, StopType> Context for CompressedCrdRdScan<ValType, StopType>
where
    ValType: DAMType
        + std::ops::AddAssign<u32>
        + std::ops::Mul<ValType, Output = ValType>
        + std::ops::Add<ValType, Output = ValType>
        + std::cmp::PartialOrd<ValType>,
    // usize: From<ValType>,
    ValType: TryInto<usize>,
    <ValType as TryInto<usize>>::Error: std::fmt::Debug,
    StopType: DAMType + std::ops::Add<u32, Output = StopType>,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        // let mut curr_crd: Token<ValType, StopType>
        loop {
            match dequeue(&mut self.time, &mut self.rd_scan_data.in_ref) {
                Ok(curr_ref) => match curr_ref.data {
                    Token::Val(val) => {
                        let idx: usize = val.try_into().unwrap();
                        // let idx: usize = usize::try_from(val).unwrap();
                        let mut curr_addr = self.seg_arr[idx];
                        let stop_addr = self.seg_arr[idx + 1];
                        while curr_addr < stop_addr {
                            let read_addr: usize = curr_addr.try_into().unwrap();
                            // let read_addr: usize = usize::try_from(curr_addr).unwrap();
                            let coord = self.crd_arr[read_addr];
                            let curr_time = self.time.tick();
                            enqueue(
                                &mut self.time,
                                &mut self.rd_scan_data.out_crd,
                                ChannelElement::new(
                                    curr_time + 1,
                                    super::primitive::Token::Val(coord),
                                ),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.rd_scan_data.out_ref,
                                ChannelElement::new(
                                    curr_time + 1,
                                    super::primitive::Token::Val(curr_addr),
                                ),
                            )
                            .unwrap();
                            curr_addr += 1;
                            self.time.incr_cycles(1);
                        }
                        let next_tkn =
                            peek_next(&mut self.time, &mut self.rd_scan_data.in_ref).unwrap();
                        let output: Token<ValType, StopType> = match next_tkn.data {
                            Token::Val(_) | Token::Done => Token::Stop(StopType::default()),
                            Token::Stop(stop_tkn) => {
                                dequeue(&mut self.time, &mut self.rd_scan_data.in_ref).unwrap();
                                Token::Stop(stop_tkn + 1)
                            }
                            Token::Empty => {
                                panic!("Invalid empty inside peek");
                            }
                        };
                        // dbg!(output);
                        let curr_time = self.time.tick();
                        enqueue(
                            &mut self.time,
                            &mut self.rd_scan_data.out_crd,
                            ChannelElement::new(curr_time + 1, output),
                        )
                        .unwrap();
                        enqueue(
                            &mut self.time,
                            &mut self.rd_scan_data.out_ref,
                            ChannelElement::new(curr_time + 1, output),
                        )
                        .unwrap();
                    }
                    Token::Stop(token) => {
                        let curr_time = self.time.tick();
                        enqueue(
                            &mut self.time,
                            &mut self.rd_scan_data.out_crd,
                            ChannelElement::new(curr_time + 1, Token::Stop(token + 1)),
                        )
                        .unwrap();
                        enqueue(
                            &mut self.time,
                            &mut self.rd_scan_data.out_ref,
                            ChannelElement::new(curr_time + 1, Token::Stop(token + 1)),
                        )
                        .unwrap();
                    }
                    // Could either be a done token or an empty token
                    // In the case of done token, return
                    tkn @ Token::Done | tkn @ Token::Empty => {
                        let channel_elem = ChannelElement::new(self.time.tick() + 1, tkn);
                        enqueue(&mut self.time, &mut self.rd_scan_data.out_crd, channel_elem)
                            .unwrap();
                        enqueue(&mut self.time, &mut self.rd_scan_data.out_ref, channel_elem)
                            .unwrap();
                        if tkn == Token::Done {
                            return;
                        }
                    }
                },
                Err(_) => panic!("Error: rd_scan_data dequeue error"),
            }
            self.time.incr_cycles(1);
        }
    }

    fn cleanup(&mut self) {
        // self.input_channels.iter_mut().for_each(|chan| {
        // chan.lock().unwrap().close();
        // });
        self.rd_scan_data.cleanup();
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

    use super::CompressedCrdRdScan;
    use super::RdScanData;
    use super::UncompressedCrdRdScan;

    #[test]
    fn ucrd_1d_test() {
        let in_ref = || [Token::Val(0u32), Token::Done].into_iter();
        let out_ref = || {
            (0u32..32)
                .map(Token::Val)
                .chain([Token::Stop(0), Token::Done])
        };
        uncompressed_rd_scan_test(in_ref, out_ref, out_ref);
    }

    #[test]
    fn ucrd_2d_test() {
        let in_ref = || {
            (0u32..4)
                .map(Token::Val)
                .chain([Token::Stop(0), Token::Done])
        };
        let out_ref = || {
            (0u32..32)
                .map(Token::Val)
                .chain([Token::Stop(0)])
                .chain((32u32..64).map(Token::Val))
                .chain([Token::Stop(0)])
                .chain((64u32..96).map(Token::Val))
                .chain([Token::Stop(0)])
                .chain((96u32..128).map(Token::Val))
                .chain([Token::Stop(1), Token::Done])
        };
        let out_crd = || {
            (0u32..32)
                .map(Token::Val)
                .chain([Token::Stop(0)])
                .cycle()
                // Repeat 3 fibers with stops and another fiber without the first level stop token since it gets replaced with second level stop
                .take(33 * 4 - 1)
                .chain([Token::Stop(1), Token::Done])
        };
        uncompressed_rd_scan_test(in_ref, out_ref, out_crd);
    }

    // #[test]
    fn uncompressed_rd_scan_test<IRT, ORT, CRT>(
        in_ref: fn() -> IRT,
        out_ref: fn() -> ORT,
        out_crd: fn() -> CRT,
    ) where
        IRT: Iterator<Item = Token<u32, u32>> + 'static,
        CRT: Iterator<Item = Token<u32, u32>> + 'static,
        ORT: Iterator<Item = Token<u32, u32>> + 'static,
    {
        let meta_dim: u32 = 32;
        let (ref_sender, ref_receiver) = unbounded::<Token<u32, u32>>();
        let (crd_sender, crd_receiver) = unbounded::<Token<u32, u32>>();
        let (in_ref_sender, in_ref_receiver) = unbounded::<Token<u32, u32>>();
        let data = RdScanData::<u32, u32> {
            in_ref: in_ref_receiver,
            out_ref: ref_sender,
            out_crd: crd_sender,
        };
        let mut ucr = UncompressedCrdRdScan::new(data, meta_dim);
        let mut gen1 = GeneratorContext::new(in_ref, in_ref_sender);
        let mut crd_checker = CheckerContext::new(out_crd, crd_receiver);
        let mut ref_checker = CheckerContext::new(out_ref, ref_receiver);
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut crd_checker);
        parent.add_child(&mut ref_checker);
        parent.add_child(&mut ucr);
        parent.init();
        parent.run();
        parent.cleanup();
    }

    #[test]
    fn crd_1d_test() {
        let seg_arr = vec![0u32, 3];
        let crd_arr = vec![0u32, 1, 3];
        let in_ref = || [Token::Val(0u32), Token::Done].into_iter();
        let out_ref = || {
            (0u32..3)
                .map(Token::Val)
                .chain([Token::Stop(0), Token::Done])
        };
        let out_crd = || {
            vec![0u32, 1, 3]
                .into_iter()
                .map(Token::Val)
                .chain([Token::Stop(0u32), Token::Done])
        };
        compressed_rd_scan_test(seg_arr, crd_arr, in_ref, out_ref, out_crd);
    }

    #[test]
    fn crd_2d_test() {
        let seg_arr = vec![0u32, 3, 4, 6];
        let crd_arr = vec![0u32, 2, 3, 0, 2, 3];
        let in_ref = || {
            [
                Token::Val(0u32),
                Token::Val(0),
                Token::Stop(0),
                Token::Val(1),
                Token::Stop(0),
                Token::Val(2),
                Token::Stop(1),
                Token::Done,
            ]
            .into_iter()
        };
        let out_ref = || {
            [0u32, 1, 2]
                .into_iter()
                .map(Token::Val)
                .chain([Token::Stop(0)])
                .chain([0u32, 1, 2].into_iter().map(Token::Val))
                .chain(
                    [
                        Token::Stop(1),
                        Token::Val(3),
                        Token::Stop(1),
                        Token::Val(4),
                        Token::Val(5),
                        Token::Stop(2),
                        Token::Done,
                    ]
                    .into_iter(),
                )
        };
        let out_crd = || {
            [0u32, 2, 3]
                .into_iter()
                .map(Token::Val)
                .chain([Token::Stop(0)])
                .chain([0u32, 2, 3].into_iter().map(Token::Val))
                .chain(
                    [
                        Token::Stop(1),
                        Token::Val(0),
                        Token::Stop(1),
                        Token::Val(2),
                        Token::Val(3),
                        Token::Stop(2),
                        Token::Done,
                    ]
                    .into_iter(),
                )
        };
        compressed_rd_scan_test(seg_arr, crd_arr, in_ref, out_ref, out_crd);
    }

    fn compressed_rd_scan_test<IRT, ORT, CRT>(
        seg_arr: Vec<u32>,
        crd_arr: Vec<u32>,
        in_ref: fn() -> IRT,
        out_ref: fn() -> ORT,
        out_crd: fn() -> CRT,
    ) where
        IRT: Iterator<Item = Token<u32, u32>> + 'static,
        CRT: Iterator<Item = Token<u32, u32>> + 'static,
        ORT: Iterator<Item = Token<u32, u32>> + 'static,
    {
        let (ref_sender, ref_receiver) = unbounded::<Token<u32, u32>>();
        let (crd_sender, crd_receiver) = unbounded::<Token<u32, u32>>();
        let (in_ref_sender, in_ref_receiver) = unbounded::<Token<u32, u32>>();
        let data = RdScanData::<u32, u32> {
            in_ref: in_ref_receiver,
            out_ref: ref_sender,
            out_crd: crd_sender,
        };
        let mut cr = CompressedCrdRdScan::new(data, seg_arr, crd_arr);
        let mut gen1 = GeneratorContext::new(in_ref, in_ref_sender);
        let mut crd_checker = CheckerContext::new(out_crd, crd_receiver);
        let mut ref_checker = CheckerContext::new(out_ref, ref_receiver);
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut crd_checker);
        parent.add_child(&mut ref_checker);
        parent.add_child(&mut cr);
        parent.init();
        parent.run();
        parent.cleanup();
    }
}
