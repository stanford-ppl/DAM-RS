use std::sync::{Arc, Mutex};

use crate::{
    channel::{
        utils::{dequeue, enqueue},
        ChannelElement, Receiver, Sender,
    },
    context::{view::TimeManager, Context},
    time::Time,
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

impl<ValType, StopType> UncompressedCrdRdScan<ValType, StopType> {
    fn new(
        rd_scan_data: RdScanData<ValType, StopType>,
        meta_dim: ValType,
    ) -> UncompressedCrdRdScan<ValType, StopType> {
        let mut ucr = UncompressedCrdRdScan {
            rd_scan_data,
            meta_dim,
            time: TimeManager::default(),
        };
        ucr.rd_scan_data.in_ref.attach_receiver(ucr);
        ucr.rd_scan_data.out_ref.attach_sender(ucr);
        ucr.rd_scan_data.out_crd.attach_sender(ucr);

        ucr
    }
}

impl<ValType, StopType> Context for UncompressedCrdRdScan<ValType, StopType>
where
    ValType: DAMType
        + std::ops::AddAssign<u32>
        + std::ops::Mul<ValType, Output = ValType>
        + std::ops::Add<ValType, Output = ValType>,
    StopType: DAMType + std::ops::Add<u32, Output = StopType>,
{
    fn init(&mut self) {}

    fn run(&mut self) -> ! {
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
                                ChannelElement::new(curr_time + 1, crd_count),
                            )
                            .unwrap();
                            crd_count += 1;
                            enqueue(
                                &mut self.time,
                                &mut self.rd_scan_data.out_ref,
                                ChannelElement::new(curr_time + 1, crd_count + val * self.meta_dim),
                            )
                            .unwrap();
                            self.time.incr_cycles(1);
                        }
                        let curr_time = self.time.tick();
                        enqueue(
                            &mut self.time,
                            &mut self.rd_scan_data.out_crd,
                            ChannelElement::new(curr_time + 1, Token::Stop(StopType::default())),
                        )
                        .unwrap();
                        enqueue(
                            &mut self.time,
                            &mut self.rd_scan_data.out_ref,
                            ChannelElement::new(curr_time + 1, Token::Stop(StopType::default())),
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
                    Token::Done => return,
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
    #[test]
    fn pcu_test() {}
}
