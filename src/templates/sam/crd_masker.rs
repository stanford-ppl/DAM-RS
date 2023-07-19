use crate::channel::utils::dequeue;
use crate::channel::utils::enqueue;
use crate::channel::utils::peek_next;
use crate::channel::ChannelElement;
use crate::channel::Receiver;
use crate::channel::Sender;
use crate::context::Context;
use crate::templates::sam::primitive::Token;
use crate::types::Cleanable;
use crate::types::DAMType;
use dam_core::identifier::Identifier;
use dam_core::TimeManager;
use dam_macros::cleanup;
use dam_macros::identifiable;
use dam_macros::time_managed;

pub struct CrdMaskData<ValType: Clone, StopType: Clone> {
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

#[time_managed]
#[identifiable]
pub struct CrdMask<ValType: Clone, StopType: Clone> {
    crd_mask_data: CrdMaskData<ValType, StopType>,
    predicate: fn(Token<ValType, StopType>, Token<ValType, StopType>) -> bool,
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
            identifier: Identifier::new(),
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
        let mut has_crd = false;
        // let icrd_vec: Vec<Token<ValType, StopType>> = Vec::new();
        loop {
            let out_ocrd = peek_next(&mut self.time, &mut self.crd_mask_data.in_crd_outer);
            match dequeue(&mut self.time, &mut self.crd_mask_data.in_crd_inner) {
                Ok(curr_in) => {
                    let curr_iref =
                        dequeue(&mut self.time, &mut self.crd_mask_data.in_ref_inner).unwrap();
                    let curr_ocrd = out_ocrd.unwrap().data.clone();
                    match curr_ocrd.clone() {
                        Token::Stop(stkn) => {
                            let channel_elem = ChannelElement::new(
                                self.time.tick() + 1,
                                Token::<ValType, StopType>::Stop(stkn.clone()),
                            );
                            enqueue(
                                &mut self.time,
                                &mut self.crd_mask_data.out_crd_outer,
                                channel_elem,
                            )
                            .unwrap();
                            dequeue(&mut self.time, &mut self.crd_mask_data.in_crd_outer).unwrap();
                        }
                        _ => (),
                    }
                    match curr_in.data {
                        Token::Val(val) => {
                            if (self.predicate)(curr_ocrd, Token::Val(val.clone())) == false {
                                let icrd_channel_elem = ChannelElement::new(
                                    self.time.tick() + 1,
                                    Token::<ValType, StopType>::Val(val.clone()),
                                );
                                enqueue(
                                    &mut self.time,
                                    &mut self.crd_mask_data.out_crd_inner,
                                    icrd_channel_elem,
                                )
                                .unwrap();
                                let iref_channel_elem = ChannelElement::new(
                                    self.time.tick() + 1,
                                    curr_iref.data.clone(),
                                );
                                enqueue(
                                    &mut self.time,
                                    &mut self.crd_mask_data.out_ref_inner,
                                    iref_channel_elem,
                                )
                                .unwrap();
                                has_crd = true;
                            }
                        }
                        Token::Stop(stkn) => {
                            if has_crd {
                                let icrd_channel_elem = ChannelElement::new(
                                    self.time.tick() + 1,
                                    Token::<ValType, StopType>::Stop(stkn.clone()),
                                );
                                enqueue(
                                    &mut self.time,
                                    &mut self.crd_mask_data.out_crd_inner,
                                    icrd_channel_elem,
                                )
                                .unwrap();
                                let iref_channel_elem = ChannelElement::new(
                                    self.time.tick() + 1,
                                    curr_iref.data.clone(),
                                );
                                enqueue(
                                    &mut self.time,
                                    &mut self.crd_mask_data.out_ref_inner,
                                    iref_channel_elem,
                                )
                                .unwrap();
                                let ocrd_channel_elem =
                                    ChannelElement::new(self.time.tick() + 1, curr_ocrd.clone());
                                enqueue(
                                    &mut self.time,
                                    &mut self.crd_mask_data.out_crd_outer,
                                    ocrd_channel_elem,
                                )
                                .unwrap();
                                has_crd = false;
                            }
                            dequeue(&mut self.time, &mut self.crd_mask_data.in_crd_outer).unwrap();
                        }
                        Token::Done => {
                            let channel_elem =
                                ChannelElement::new(self.time.tick() + 1, Token::Done);
                            enqueue(
                                &mut self.time,
                                &mut self.crd_mask_data.out_crd_inner,
                                channel_elem.clone(),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.crd_mask_data.out_ref_inner,
                                channel_elem.clone(),
                            )
                            .unwrap();
                            enqueue(
                                &mut self.time,
                                &mut self.crd_mask_data.out_crd_outer,
                                channel_elem.clone(),
                            )
                            .unwrap();
                            return;
                        }
                        _ => {
                            dbg!(curr_in.data.clone());
                            panic!("Invalid case found");
                        }
                    }
                }
                Err(_) => {
                    panic!("Error encountered!");
                }
            }
            self.time.incr_cycles(1);
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.crd_mask_data.cleanup();
        self.time.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        channel::unbounded,
        context::{
            checker_context::CheckerContext, generator_context::GeneratorContext,
            parent::BasicParentContext, Context,
        },
        templates::sam::primitive::Token,
        token_vec,
    };

    use super::{CrdMask, CrdMaskData};

    #[test]
    fn test_tril_mask() {
        let in_crd_outer = || token_vec!(u32; u32; 0, 1, 2, "S0", "D").into_iter();
        let in_crd_inner =
            || token_vec!(u32; u32; 0, 1, 3, "S0", 0, 1, 2, "S0", 0, 1, 2, "S1", "D").into_iter();
        let in_ref_inner =
            || token_vec!(u32; u32; 0, 1, 2, "S0", 3, 4, 5, "S0", 6, 7, 8, "S1", "D").into_iter();

        let out_crd_outer = || token_vec!(u32; u32; 0, 1, 2, "S0", "D").into_iter();
        let out_crd_inner =
            || token_vec!(u32; u32; 0, 1, 3, "S0", 1, 2, "S0", 2, "S1", "D").into_iter();
        let out_ref_inner =
            || token_vec!(u32; u32; 0, 1, 2, "S0", 4, 5, "S0", 8, "S1", "D").into_iter();
        mask_test(
            in_crd_outer,
            in_crd_inner,
            in_ref_inner,
            out_crd_outer,
            out_crd_inner,
            out_ref_inner,
        );
    }

    #[test]
    fn test_tril_mask1() {
        let in_crd_outer = || token_vec!(u32; u32; 4, 1, 2, "S0", "D").into_iter();
        let in_crd_inner =
            || token_vec!(u32; u32; 0, 1, 3, "S0", 0, 1, 2, "S0", 0, 1, 2, "S1", "D").into_iter();
        let in_ref_inner =
            || token_vec!(u32; u32; 0, 1, 2, "S0", 3, 4, 5, "S0", 6, 7, 8, "S1", "D").into_iter();

        let out_crd_outer = || token_vec!(u32; u32; 1, 2, "S0", "D").into_iter();
        let out_crd_inner = || token_vec!(u32; u32; 1, 2, "S0", 2, "S1", "D").into_iter();
        let out_ref_inner = || token_vec!(u32; u32;  4, 5, "S0", 8, "S1", "D").into_iter();
        mask_test(
            in_crd_outer,
            in_crd_inner,
            in_ref_inner,
            out_crd_outer,
            out_crd_inner,
            out_ref_inner,
        );
    }

    fn mask_test<IRT1, IRT2, IRT3, ORT1, ORT2, ORT3>(
        in_crd_outer: fn() -> IRT2,
        in_crd_inner: fn() -> IRT1,
        in_ref_inner: fn() -> IRT3,
        out_crd_outer: fn() -> ORT2,
        out_crd_inner: fn() -> ORT1,
        out_ref_inner: fn() -> ORT3,
    ) where
        IRT1: Iterator<Item = Token<u32, u32>> + 'static,
        IRT2: Iterator<Item = Token<u32, u32>> + 'static,
        IRT3: Iterator<Item = Token<u32, u32>> + 'static,
        ORT1: Iterator<Item = Token<u32, u32>> + 'static,
        ORT2: Iterator<Item = Token<u32, u32>> + 'static,
        ORT3: Iterator<Item = Token<u32, u32>> + 'static,
    {
        let (mask_in_crd_sender, mask_in_crd_receiver) = unbounded::<Token<u32, u32>>();
        let (mask_in_ocrd_sender, mask_in_ocrd_receiver) = unbounded::<Token<u32, u32>>();
        let (mask_in_ref_sender, mask_in_ref_receiver) = unbounded::<Token<u32, u32>>();

        let (mask_out_crd_sender, mask_out_crd_receiver) = unbounded::<Token<u32, u32>>();
        let (mask_out_ocrd_sender, mask_out_ocrd_receiver) = unbounded::<Token<u32, u32>>();
        let (mask_out_ref_sender, mask_out_ref_receiver) = unbounded::<Token<u32, u32>>();

        let mut gen1 = GeneratorContext::new(in_crd_outer, mask_in_ocrd_sender);
        let mut gen2 = GeneratorContext::new(in_crd_inner, mask_in_crd_sender);
        let mut gen3 = GeneratorContext::new(in_ref_inner, mask_in_ref_sender);
        let mut ocrd_checker = CheckerContext::new(out_crd_outer, mask_out_ocrd_receiver);
        let mut icrd_checker = CheckerContext::new(out_crd_inner, mask_out_crd_receiver);
        let mut iref_checker = CheckerContext::new(out_ref_inner, mask_out_ref_receiver);

        let mask_data = CrdMaskData::<u32, u32> {
            in_crd_inner: mask_in_crd_receiver,
            in_ref_inner: mask_in_ref_receiver,
            in_crd_outer: mask_in_ocrd_receiver,
            out_crd_inner: mask_out_crd_sender,
            out_crd_outer: mask_out_ocrd_sender,
            out_ref_inner: mask_out_ref_sender,
        };
        let mut mask = CrdMask::new(mask_data, |x, y| x > y);
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut gen2);
        parent.add_child(&mut gen3);
        parent.add_child(&mut ocrd_checker);
        parent.add_child(&mut icrd_checker);
        parent.add_child(&mut iref_checker);
        parent.add_child(&mut mask);
        parent.init();
        parent.run();
        parent.cleanup();
    }
}
