use dam_core::identifier::Identifier;
use dam_core::TimeManager;
use dam_macros::{cleanup, identifiable, time_managed};

use crate::{
    channel::{
        utils::{dequeue, enqueue, Peekable},
        ChannelElement, Receiver, Sender,
    },
    context::Context,
    types::{Cleanable, DAMType},
};

use super::primitive::Token;

pub struct ValDropData<CrdType, ValType, StopType> {
    pub in_val: Receiver<Token<ValType, StopType>>,
    pub in_crd: Receiver<Token<CrdType, StopType>>,
    pub out_val: Sender<Token<ValType, StopType>>,
    pub out_crd: Sender<Token<CrdType, StopType>>,
}

impl<CrdType: DAMType, ValType: DAMType, StopType: DAMType> Cleanable
    for ValDropData<CrdType, ValType, StopType>
{
    fn cleanup(&mut self) {
        self.in_val.cleanup();
        self.in_crd.cleanup();
        self.out_val.cleanup();
        self.out_crd.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct ValDrop<CrdType, ValType, StopType> {
    val_drop_data: ValDropData<CrdType, ValType, StopType>,
}

impl<CrdType: DAMType, ValType: DAMType, StopType: DAMType> ValDrop<CrdType, ValType, StopType>
where
    ValDrop<CrdType, ValType, StopType>: Context,
{
    pub fn new(array_data: ValDropData<CrdType, ValType, StopType>) -> Self {
        let val_drop = ValDrop {
            val_drop_data: array_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (val_drop.val_drop_data.in_val).attach_receiver(&val_drop);
        (val_drop.val_drop_data.in_crd).attach_receiver(&val_drop);
        (val_drop.val_drop_data.out_val).attach_sender(&val_drop);
        (val_drop.val_drop_data.out_crd).attach_sender(&val_drop);

        val_drop
    }
}

impl<CrdType, ValType, StopType> Context for ValDrop<CrdType, ValType, StopType>
where
    CrdType: DAMType + std::cmp::PartialEq + std::cmp::PartialOrd,
    ValType: DAMType + std::cmp::PartialEq + std::cmp::PartialOrd,
    StopType: DAMType + std::ops::Add<u32, Output = StopType> + std::cmp::PartialEq,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        let mut prev_stkn = false;
        loop {
            let _ = self.val_drop_data.in_val.next_event();
            let _ = self.val_drop_data.in_crd.next_event();

            let val_deq = dequeue(&mut self.time, &mut self.val_drop_data.in_val);
            let crd_deq = dequeue(&mut self.time, &mut self.val_drop_data.in_crd);
            match (val_deq, crd_deq) {
                (Ok(val), Ok(crd)) => match (val.data, crd.data) {
                    (Token::Val(value), Token::Val(coord)) if value != ValType::default() => {
                        let val_chan_elem = ChannelElement::new(
                            self.time.tick() + 1,
                            Token::<ValType, StopType>::Val(value),
                        );
                        enqueue(
                            &mut self.time,
                            &mut self.val_drop_data.out_val,
                            val_chan_elem,
                        )
                        .unwrap();
                        let crd_chan_elem = ChannelElement::new(
                            self.time.tick() + 1,
                            Token::<CrdType, StopType>::Val(coord),
                        );
                        enqueue(
                            &mut self.time,
                            &mut self.val_drop_data.out_crd,
                            crd_chan_elem,
                        )
                        .unwrap();
                    }
                    (Token::Val(val), Token::Val(_)) if val == ValType::default() => (),
                    (tkn1 @ Token::Stop(_), tkn2 @ Token::Stop(_))
                    | (tkn1 @ Token::Done, tkn2 @ Token::Done) => {
                        if tkn1 != Token::Done {
                            if prev_stkn {
                                prev_stkn = false;
                                continue;
                            }
                        }
                        let val_chan_elem = ChannelElement::new(self.time.tick() + 1, tkn1.clone());
                        enqueue(
                            &mut self.time,
                            &mut self.val_drop_data.out_val,
                            val_chan_elem,
                        )
                        .unwrap();
                        let crd_chan_elem = ChannelElement::new(self.time.tick() + 1, tkn2.clone());
                        enqueue(
                            &mut self.time,
                            &mut self.val_drop_data.out_crd,
                            crd_chan_elem,
                        )
                        .unwrap();
                        if tkn1 == Token::Done {
                            return;
                        } else {
                            prev_stkn = true;
                        }
                    }
                    _ => {
                        panic!("Invalid case reached in val_dropper");
                    }
                },
                _ => {
                    panic!("dequeue error in val, crd match");
                }
            }
            self.time.incr_cycles(1);
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.val_drop_data.cleanup();
        self.time.cleanup();
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
        token_vec,
    };

    use super::ValDrop;
    use super::ValDropData;

    #[test]
    fn val_drop_2d_test() {
        let in_val = || {
            token_vec![f32; u32; 0.0, 1.0, 2.0, "S0", 0.0, "S0", 2.0, 3.0, 4.0, "S1", "D"]
                .into_iter()
        };
        let in_crd =
            || token_vec![u32; u32; 0, 1, 2, "S0", 0, "S0", 2, 3, 4, "S1", "D"].into_iter();
        let out_val = || token_vec![f32; u32; 1.0, 2.0, "S0", 2.0, 3.0, 4.0, "S1", "D"].into_iter();
        let out_crd = || token_vec![u32; u32; 1, 2, "S0", 2, 3, 4, "S1", "D"].into_iter();
        val_drop_test(in_val, in_crd, out_val, out_crd);
    }

    fn val_drop_test<IRT1, IRT2, ORT1, ORT2>(
        in_val: fn() -> IRT1,
        in_crd: fn() -> IRT2,
        out_val: fn() -> ORT1,
        out_crd: fn() -> ORT2,
    ) where
        IRT1: Iterator<Item = Token<f32, u32>> + 'static,
        IRT2: Iterator<Item = Token<u32, u32>> + 'static,
        ORT1: Iterator<Item = Token<f32, u32>> + 'static,
        ORT2: Iterator<Item = Token<u32, u32>> + 'static,
    {
        let (in_val_sender, in_val_receiver) = unbounded::<Token<f32, u32>>();
        let (in_crd_sender, in_crd_receiver) = unbounded::<Token<u32, u32>>();
        let (out_val_sender, out_val_receiver) = unbounded::<Token<f32, u32>>();
        let (out_crd_sender, out_crd_receiver) = unbounded::<Token<u32, u32>>();
        let data = ValDropData::<u32, f32, u32> {
            in_val: in_val_receiver,
            in_crd: in_crd_receiver,
            out_val: out_val_sender,
            out_crd: out_crd_sender,
        };
        let mut val_drop = ValDrop::new(data);
        let mut gen1 = GeneratorContext::new(in_val, in_val_sender);
        let mut gen2 = GeneratorContext::new(in_crd, in_crd_sender);
        let mut out_val_checker = CheckerContext::new(out_val, out_val_receiver);
        let mut out_crd_checker = CheckerContext::new(out_crd, out_crd_receiver);
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut gen2);
        parent.add_child(&mut out_val_checker);
        parent.add_child(&mut out_crd_checker);
        parent.add_child(&mut val_drop);
        parent.init();
        parent.run();
        parent.cleanup();
    }
}
