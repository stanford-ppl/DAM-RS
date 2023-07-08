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

use super::primitive::{Repsiggen, Token};

pub struct RepeatData<ValType, StopType> {
    pub in_ref: Receiver<Token<ValType, StopType>>,
    pub in_repsig: Receiver<Repsiggen>,
    pub out_ref: Sender<Token<ValType, StopType>>,
}

impl<ValType: DAMType, StopType: DAMType> Cleanable for RepeatData<ValType, StopType> {
    fn cleanup(&mut self) {
        self.in_ref.cleanup();
        self.in_repsig.cleanup();
        self.out_ref.cleanup();
    }
}

pub struct Repeat<ValType, StopType> {
    repeat_data: RepeatData<ValType, StopType>,
    time: TimeManager,
}

impl<ValType: DAMType, StopType: DAMType> Repeat<ValType, StopType>
where
    Repeat<ValType, StopType>: Context,
{
    pub fn new(repeat_data: RepeatData<ValType, StopType>) -> Self {
        let repeat = Repeat {
            repeat_data,
            time: TimeManager::default(),
        };
        (repeat.repeat_data.in_ref).attach_receiver(&repeat);
        (repeat.repeat_data.in_repsig).attach_receiver(&repeat);
        (repeat.repeat_data.out_ref).attach_sender(&repeat);

        repeat
    }
}

impl<ValType, StopType> Context for Repeat<ValType, StopType>
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
            let in_ref = peek_next(&mut self.time, &mut self.repeat_data.in_ref);
            match dequeue(&mut self.time, &mut self.repeat_data.in_repsig) {
                Ok(curr_in) => {
                    let curr_ref = in_ref.unwrap().data;
                    match curr_in.data {
                        Repsiggen::Repeat => {
                            let channel_elem = ChannelElement::new(self.time.tick() + 1, curr_ref);
                            enqueue(&mut self.time, &mut self.repeat_data.out_ref, channel_elem)
                                .unwrap()
                        }
                        Repsiggen::Stop => {
                            dequeue(&mut self.time, &mut self.repeat_data.in_ref).unwrap();
                            let next_tkn =
                                peek_next(&mut self.time, &mut self.repeat_data.in_ref).unwrap();
                            let output: Token<ValType, StopType> = match next_tkn.data {
                                Token::Val(_) | Token::Empty => Token::Stop(StopType::default()),
                                Token::Stop(stop_tkn) => {
                                    dequeue(&mut self.time, &mut self.repeat_data.in_ref).unwrap();
                                    Token::Stop(stop_tkn + 1)
                                }
                                Token::Done => {
                                    panic!("Invalid done token found");
                                }
                            };
                            let curr_time = self.time.tick();
                            enqueue(
                                &mut self.time,
                                &mut self.repeat_data.out_ref,
                                ChannelElement::new(curr_time + 1, output.clone()),
                            )
                            .unwrap();
                        }
                        Repsiggen::Done => {
                            // let next_tkn =
                            // peek_next(&mut self.time, &mut self.repeat_data.in_ref).unwrap();
                            match curr_ref {
                                Token::Done => {
                                    let channel_elem =
                                        ChannelElement::new(self.time.tick() + 1, Token::Done);
                                    enqueue(
                                        &mut self.time,
                                        &mut self.repeat_data.out_ref,
                                        channel_elem,
                                    )
                                    .unwrap();
                                }
                                _ => {
                                    panic!(
                                        "Input reference and repeat signal must both be on Done"
                                    );
                                }
                            }
                            return;
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
        self.repeat_data.cleanup();
        self.time.cleanup();
    }

    fn view(&self) -> TimeView {
        self.time.view().into()
    }
}

pub struct RepSigGenData<ValType, StopType> {
    pub input: Receiver<Token<ValType, StopType>>,
    pub out_repsig: Sender<Repsiggen>,
}

impl<ValType: DAMType, StopType: DAMType> Cleanable for RepSigGenData<ValType, StopType> {
    fn cleanup(&mut self) {
        self.input.cleanup();
        self.out_repsig.cleanup();
    }
}

pub struct RepeatSigGen<ValType, StopType> {
    rep_sig_gen_data: RepSigGenData<ValType, StopType>,
    time: TimeManager,
}

impl<ValType: DAMType, StopType: DAMType> RepeatSigGen<ValType, StopType>
where
    RepeatSigGen<ValType, StopType>: Context,
{
    pub fn new(rep_sig_gen_data: RepSigGenData<ValType, StopType>) -> Self {
        let rep_sig_gen = RepeatSigGen {
            rep_sig_gen_data,
            time: TimeManager::default(),
        };
        (rep_sig_gen.rep_sig_gen_data.input).attach_receiver(&rep_sig_gen);
        (rep_sig_gen.rep_sig_gen_data.out_repsig).attach_sender(&rep_sig_gen);

        rep_sig_gen
    }
}

impl<ValType, StopType> Context for RepeatSigGen<ValType, StopType>
where
    ValType: DAMType
        + std::ops::Mul<ValType, Output = ValType>
        + std::ops::Add<ValType, Output = ValType>
        + std::cmp::PartialOrd<ValType>,
    StopType: DAMType + std::ops::Add<u32, Output = StopType>,
    Repsiggen: DAMType,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            match dequeue(&mut self.time, &mut self.rep_sig_gen_data.input) {
                Ok(curr_in) => match curr_in.data {
                    Token::Val(_) | Token::Empty => {
                        let channel_elem =
                            ChannelElement::new(self.time.tick() + 1, Repsiggen::Repeat);
                        enqueue(
                            &mut self.time,
                            &mut self.rep_sig_gen_data.out_repsig,
                            channel_elem,
                        )
                        .unwrap();
                        dbg!(Repsiggen::Repeat);
                    }
                    Token::Stop(_) => {
                        let channel_elem =
                            ChannelElement::new(self.time.tick() + 1, Repsiggen::Stop);
                        enqueue(
                            &mut self.time,
                            &mut self.rep_sig_gen_data.out_repsig,
                            channel_elem,
                        )
                        .unwrap();
                        dbg!(Repsiggen::Stop);
                    }
                    Token::Done => {
                        let channel_elem =
                            ChannelElement::new(self.time.tick() + 1, Repsiggen::Done);
                        enqueue(
                            &mut self.time,
                            &mut self.rep_sig_gen_data.out_repsig,
                            channel_elem,
                        )
                        .unwrap();
                        dbg!(Repsiggen::Done);
                        return;
                    }
                },
                Err(_) => {
                    panic!("Unexpected end of stream");
                }
            }
            self.time.incr_cycles(1);
        }
    }

    fn cleanup(&mut self) {
        self.rep_sig_gen_data.cleanup();
        self.time.cleanup();
    }

    fn view(&self) -> TimeView {
        self.time.view().into()
    }
}

#[cfg(test)]
mod tests {
    use crate::templates::sam::repeat::Repsiggen;
    use crate::{
        channel::unbounded,
        context::{
            checker_context::CheckerContext, generator_context::GeneratorContext,
            parent::BasicParentContext, Context, ParentContext,
        },
        repsig_vec,
        templates::sam::primitive::Token,
        token_vec,
    };

    use super::RepSigGenData;
    use super::Repeat;
    use super::RepeatData;
    use super::RepeatSigGen;

    #[test]
    fn repeat_2d_test() {
        let in_ref = || token_vec!(u32; u32; 0, 1, "S0", 2, "S0", 3, "S1", "D").into_iter();
        let in_repsig = || {
            repsig_vec!("R", "R", "R", "S", "R", "R", "R", "S", "R", "S", "R", "R", "S", "D")
                .into_iter()
        };
        let out_ref = || {
            token_vec!(u32; u32; 0, 0, 0, "S0", 1, 1, 1, "S1", 2, "S1", 3, 3, "S2", "D").into_iter()
        };
        repeat_test(in_ref, in_repsig, out_ref);
    }

    #[test]
    fn repsiggen_2d_test() {
        let in_ref = || token_vec!(u32; u32; 0, 1, "S0", 2, "S0", 3, "S1", "D").into_iter();
        let out_repsig = || repsig_vec!("R", "R", "S", "R", "S", "R", "S", "D").into_iter();
        repsiggen_test(in_ref, out_repsig);
    }

    fn repeat_test<IRT1, IRT2, ORT>(
        in_ref: fn() -> IRT1,
        in_repsig: fn() -> IRT2,
        out_ref: fn() -> ORT,
    ) where
        IRT1: Iterator<Item = Token<u32, u32>> + 'static,
        IRT2: Iterator<Item = Repsiggen> + 'static,
        ORT: Iterator<Item = Token<u32, u32>> + 'static,
    {
        let (in_ref_sender, in_ref_receiver) = unbounded::<Token<u32, u32>>();
        let (in_repsig_sender, in_repsig_receiver) = unbounded::<Repsiggen>();
        let (out_ref_sender, out_ref_receiver) = unbounded::<Token<u32, u32>>();
        let data = RepeatData::<u32, u32> {
            in_ref: in_ref_receiver,
            in_repsig: in_repsig_receiver,
            out_ref: out_ref_sender,
        };
        let mut rep = Repeat::new(data);
        let mut gen1 = GeneratorContext::new(in_ref, in_ref_sender);
        let mut gen2 = GeneratorContext::new(in_repsig, in_repsig_sender);
        let mut val_checker = CheckerContext::new(out_ref, out_ref_receiver);
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut gen2);
        parent.add_child(&mut val_checker);
        parent.add_child(&mut rep);
        parent.init();
        parent.run();
        parent.cleanup();
    }

    fn repsiggen_test<IRT, ORT>(in_ref: fn() -> IRT, out_repsig: fn() -> ORT)
    where
        IRT: Iterator<Item = Token<u32, u32>> + 'static,
        ORT: Iterator<Item = Repsiggen> + 'static,
    {
        let (in_ref_sender, in_ref_receiver) = unbounded::<Token<u32, u32>>();
        let (out_repsig_sender, out_repsig_receiver) = unbounded::<Repsiggen>();
        let data = RepSigGenData::<u32, u32> {
            input: in_ref_receiver,
            out_repsig: out_repsig_sender,
        };

        let mut repsig = RepeatSigGen::new(data);
        let mut gen1 = GeneratorContext::new(in_ref, in_ref_sender);
        let mut val_checker = CheckerContext::new(out_repsig, out_repsig_receiver);
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut val_checker);
        parent.add_child(&mut repsig);
        parent.init();
        parent.run();
        parent.cleanup();
    }
}
