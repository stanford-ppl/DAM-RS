use dam_core::identifier::Identifier;
use dam_core::TimeManager;
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

#[time_managed]
#[identifiable]
pub struct StknDrop<ValType: Clone, StopType: Clone> {
    pub in_val: Receiver<Token<ValType, StopType>>,
    pub out_val: Sender<Token<ValType, StopType>>,
}

impl<ValType: DAMType, StopType: DAMType> StknDrop<ValType, StopType>
where
    StknDrop<ValType, StopType>: Context,
{
    pub fn new(
        in_val: Receiver<Token<ValType, StopType>>,
        out_val: Sender<Token<ValType, StopType>>,
    ) -> Self {
        let stkn_drop = StknDrop {
            in_val,
            out_val,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (stkn_drop).in_val.attach_receiver(&stkn_drop);
        (stkn_drop).out_val.attach_sender(&stkn_drop);

        stkn_drop
    }
}

impl<ValType, StopType> Context for StknDrop<ValType, StopType>
where
    ValType: DAMType + std::cmp::PartialEq + std::cmp::PartialOrd,
    StopType: DAMType + std::ops::Add<u32, Output = StopType> + std::cmp::PartialEq,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        let mut prev_stkn = false;
        loop {
            let val_deq = dequeue(&mut self.time, &mut self.in_val);
            match val_deq {
                Ok(curr_in) => match curr_in.data {
                    tkn @ Token::Val(_) | tkn @ Token::Done => {
                        let channel_elem = ChannelElement::new(self.time.tick() + 1, tkn.clone());
                        enqueue(&mut self.time, &mut self.out_val, channel_elem).unwrap();
                        if tkn == Token::Done {
                            return;
                        }
                        prev_stkn = false;
                    }
                    Token::Stop(stkn) => {
                        if !prev_stkn {
                            let channel_elem = ChannelElement::new(
                                self.time.tick() + 1,
                                Token::<ValType, StopType>::Stop(stkn),
                            );
                            enqueue(&mut self.time, &mut self.out_val, channel_elem).unwrap();
                            prev_stkn = true;
                        }
                    }
                    _ => {
                        panic!("Invalid token found in stream");
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
        self.in_val.cleanup();
        self.out_val.cleanup();
        self.time.cleanup();
    }
}
