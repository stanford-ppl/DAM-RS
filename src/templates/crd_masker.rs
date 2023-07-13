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

use super::sam::primitive::Token;

pub struct CrdMaskData<ValType, StopType> {
    pub in_crd_inner: Receiver<Token<ValType, StopType>>,
    pub in_crd_outer: Receiver<Token<ValType, StopType>>,
    pub out_crd_inner: Sender<Token<ValType, StopType>>,
    pub out_crd_outer: Sender<Token<ValType, StopType>>,
    pub in_ref_inner: Receiver<Token<ValType, StopType>>,
    pub out_ref_inner: Sender<Token<ValType, StopType>>,
}

impl<ValType: DAMType, StopType: DAMType> Cleanable for CrdMaskData<ValType, StopType> {
    fn cleanup(&mut self) {
        self.in_crd_inner.cleanup();
        self.in_crd_outer.cleanup();
        self.in_ref_inner.cleanup();
        self.out_crd_inner.cleanup();
        self.out_crd_outer.cleanup();
        self.out_ref_inner.cleanup();
    }
}

pub struct CrdMask<ValType, StopType> {
    crd_mask_data: CrdMaskData<ValType, StopType>,
    predicate: fn(Token<ValType, StopType>, Token<ValType, StopType>) -> bool,
    time: TimeManager,
}

impl<ValType: DAMType, StopType: DAMType> CrdMask<ValType, StopType>
where
    CrdMask<ValType, StopType>: Context,
{
    pub fn new(
        crd_mask_data: CrdMaskData<ValType, StopType>,
        predicate: fn(Token<ValType, StopType>, Token<ValType, StopType>) -> bool,
    ) -> Self {
        let mask = CrdMask {
            crd_mask_data,
            predicate,
            time: TimeManager::default(),
        };
        (mask.crd_mask_data.in_crd_inner).attach_receiver(&mask);
        (mask.crd_mask_data.in_crd_outer).attach_receiver(&mask);
        (mask.crd_mask_data.in_ref_inner).attach_receiver(&mask);
        (mask.crd_mask_data.out_crd_inner).attach_sender(&mask);
        (mask.crd_mask_data.out_crd_outer).attach_sender(&mask);
        (mask.crd_mask_data.out_ref_inner).attach_sender(&mask);

        mask
    }
}

impl<ValType, StopType> Context for CrdMask<ValType, StopType>
where
    ValType: DAMType
        + std::ops::Mul<ValType, Output = ValType>
        + std::ops::Add<ValType, Output = ValType>
        + std::cmp::PartialOrd<ValType>,
    StopType: DAMType + std::ops::Add<u32, Output = StopType>,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        // let mut get_next_ocrd = false;
        let mut has_crd = false;
        // let icrd_vec: Vec<Token<ValType, StopType>> = Vec::new();
        loop {
            let out_ocrd = peek_next(&mut self.time, &mut self.crd_mask_data.in_crd_outer);
            match dequeue(&mut self.time, &mut self.crd_mask_data.in_crd_inner) {
                Ok(curr_in) => {
                    let curr_iref =
                        dequeue(&mut self.time, &mut self.crd_mask_data.in_ref_inner).unwrap();
                    let curr_ocrd = out_ocrd.unwrap().data.clone();
                    match curr_in.data {
                        Token::Val(val) => {
                            if (self.predicate)(curr_ocrd, Token::Val(val.clone())) == false {
                                let channel_elem = ChannelElement::new(
                                    self.time.tick() + 1,
                                    Token::<ValType, StopType>::Val(val.clone()),
                                );
                                // enqueue(&mut self.time(), self.crd_mask_data.o, data)
                            }
                        }
                        Token::Stop(_) => todo!(),
                        Token::Empty => todo!(),
                        Token::Done => todo!(),
                    }
                }
                Err(_) => todo!(),
            }
            self.time.incr_cycles(1);
        }
    }

    fn cleanup(&mut self) {
        self.crd_mask_data.cleanup();
        self.time.cleanup();
    }

    fn view(&self) -> TimeView {
        self.time.view().into()
    }
}

#[cfg(test)]
mod tests {
    use crate::{channel::unbounded, templates::sam::primitive::Token};

    use super::{CrdMask, CrdMaskData};

    #[test]
    fn test_fn() {
        let (mask_in_crd_sender, mask_in_crd_receiver) = unbounded::<Token<u32, u32>>();
        let (mask_in_ocrd_sender, mask_in_ocrd_receiver) = unbounded::<Token<u32, u32>>();
        let (mask_in_ref_sender, mask_in_ref_receiver) = unbounded::<Token<u32, u32>>();

        let (mask_out_crd_sender, mask_out_crd_receiver) = unbounded::<Token<u32, u32>>();
        let (mask_out_ocrd_sender, mask_out_ocrd_receiver) = unbounded::<Token<u32, u32>>();
        let (mask_out_ref_sender, mask_out_ref_receiver) = unbounded::<Token<u32, u32>>();
        let mask_data = CrdMaskData::<u32, u32> {
            in_crd_inner: mask_in_crd_receiver,
            in_ref_inner: mask_in_ref_receiver,
            in_crd_outer: mask_in_ocrd_receiver,
            out_crd_inner: mask_out_crd_sender,
            out_crd_outer: mask_out_ocrd_sender,
            out_ref_inner: mask_out_ref_sender,
        };
        let mut mask = CrdMask::new(mask_data, |x, y| x < y);
    }
}
