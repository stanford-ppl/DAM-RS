use core::panic;

use crate::{
    channel::{
        utils::{dequeue, enqueue, peek_next},
        ChannelElement, Receiver, Sender,
    },
    context::{
        view::{TimeManager, TimeView},
        Context,
    },
    types::{Cleanable, DAMType},
};

use super::primitive::Token;

pub struct CrdDropData<ValType, StopType> {
    pub in_crd_inner: Receiver<Token<ValType, StopType>>,
    pub in_crd_outer: Receiver<Token<ValType, StopType>>,
    pub out_crd_inner: Sender<Token<ValType, StopType>>,
    pub out_crd_outer: Sender<Token<ValType, StopType>>,
}

impl<ValType: DAMType, StopType: DAMType> Cleanable for CrdDropData<ValType, StopType> {
    fn cleanup(&mut self) {
        self.in_crd_inner.cleanup();
        self.in_crd_outer.cleanup();
        self.out_crd_inner.cleanup();
        self.out_crd_outer.cleanup();
    }
}

pub struct CrdDrop<ValType, StopType> {
    crd_drop_data: CrdDropData<ValType, StopType>,
    time: TimeManager,
}

impl<ValType: DAMType, StopType: DAMType> CrdDrop<ValType, StopType>
where
    CrdDrop<ValType, StopType>: Context,
{
    pub fn new(crd_drop_data: CrdDropData<ValType, StopType>) -> Self {
        let drop = CrdDrop {
            crd_drop_data,
            time: TimeManager::default(),
        };
        (drop.crd_drop_data.in_crd_inner).attach_receiver(&drop);
        (drop.crd_drop_data.in_crd_outer).attach_receiver(&drop);
        (drop.crd_drop_data.out_crd_inner).attach_sender(&drop);
        (drop.crd_drop_data.out_crd_outer).attach_sender(&drop);

        drop
    }
}

impl<ValType, StopType> Context for CrdDrop<ValType, StopType>
where
    ValType: DAMType
        + std::ops::Mul<ValType, Output = ValType>
        + std::ops::Add<ValType, Output = ValType>
        + std::cmp::PartialOrd<ValType>,
    StopType: DAMType + std::ops::Add<u32, Output = StopType>,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        let mut get_next_ocrd = false;
        let mut has_crd = false;
        loop {
            // if get_next_ocrd {
            //     dequeue(&mut self.time, &mut self.crd_drop_data.in_crd_outer);
            // }
            let out_ocrd = peek_next(&mut self.time, &mut self.crd_drop_data.in_crd_outer);
            match dequeue(&mut self.time, &mut self.crd_drop_data.in_crd_inner) {
                Ok(curr_in) => {
                    let curr_ocrd = out_ocrd.unwrap().data;
                    match curr_in.data {
                        Token::Val(_) => {
                            has_crd = true;
                            continue;
                        }
                        Token::Stop(stkn) => {
                            let c_ocrd =
                                dequeue(&mut self.time, &mut self.crd_drop_data.in_crd_outer)
                                    .unwrap();
                            match c_ocrd.data {
                                Token::Val(_) => {
                                    continue;
                                }
                                Token::Stop(tkn) => {
                                    let channel_elem =
                                        ChannelElement::new(self.time.tick() + 1, Token::Stop(tkn));
                                    enqueue(
                                        &mut self.time,
                                        &mut self.crd_drop_data.out_crd_outer,
                                        channel_elem,
                                    )
                                    .unwrap();
                                }
                                _ => {
                                    panic!("Invalid token reached");
                                }
                            }
                        }
                        Token::Done => {
                            let channel_elem =
                                ChannelElement::new(self.time.tick() + 1, Token::Done);
                            enqueue(
                                &mut self.time,
                                &mut self.crd_drop_data.out_crd_outer,
                                channel_elem.clone(),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.crd_drop_data.out_crd_inner,
                                channel_elem,
                            )
                            .unwrap();
                        }
                        _ => {
                            panic!("Invalid token reached");
                        }
                    }
                }
                Err(_) => {
                    panic!("Unexpected end of stream");
                }
            }
            self.time.incr_cycles(1);
        }
    }

    fn cleanup(&mut self) {
        self.crd_drop_data.cleanup();
        self.time.cleanup();
    }

    fn view(&self) -> TimeView {
        self.time.view().into()
    }
}
