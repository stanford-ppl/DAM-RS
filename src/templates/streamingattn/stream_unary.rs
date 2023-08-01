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
pub struct UnaryScalarData<A: Clone> {
    pub in_stream: Receiver<A>,   // operand 1: scalar
    pub out_list: Vec<Sender<A>>, // output -> Scalar FIFO
    pub latency: u64,             // pipeline depth
    pub init_inverval: u64,       // initiation interval
    pub loop_bound: u64,
}

impl<A: DAMType> Cleanable for UnaryScalarData<A> {
    fn cleanup(&mut self) {
        self.in_stream.cleanup();
        for i in self.out_list.iter_mut() {
            i.cleanup();
        }
    }
}

pub enum UnaryOpType {
    Exp,
}

#[time_managed]
#[identifiable]
pub struct UnaryScalarOp<A: Clone> {
    unary_data: UnaryScalarData<A>,
    op: UnaryOpType,
}

impl<A: DAMType> UnaryScalarOp<A>
where
    UnaryScalarOp<A>: Context,
{
    pub fn new(unary_data: UnaryScalarData<A>, op: UnaryOpType) -> Self {
        let unary_op = UnaryScalarOp {
            unary_data,
            op,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (unary_op.unary_data.in_stream).attach_receiver(&unary_op);
        for i in unary_op.unary_data.out_list.iter() {
            i.attach_sender(&unary_op);
        }

        unary_op
    }
}

impl<A> Context for UnaryScalarOp<A>
where
    A: DAMType + num::Float,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        for _i in 0..self.unary_data.loop_bound {
            let in_deq: Result<ChannelElement<_>, crate::channel::DequeueError> =
                dequeue(&mut self.time, &mut self.unary_data.in_stream);

            match in_deq {
                Ok(in_elem) => {
                    let in_data = in_elem.data;
                    let out_data: A;
                    match self.op {
                        UnaryOpType::Exp => {
                            out_data = in_data.exp();
                        }
                    }

                    let curr_time = self.time.tick();
                    for mut j in self.unary_data.out_list.iter_mut() {
                        enqueue(
                            &mut self.time,
                            &mut j,
                            ChannelElement::new(
                                curr_time + self.unary_data.latency,
                                out_data.clone(),
                            ),
                        )
                        .unwrap();
                    }
                }
                _ => {
                    panic!("Reached unhandled case");
                }
            }
            self.time.incr_cycles(self.unary_data.init_inverval);
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.unary_data.cleanup();
        self.time.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        context::{checker_context::CheckerContext, generator_context::GeneratorContext},
        simulation::Program,
    };

    use super::{UnaryOpType, UnaryScalarData, UnaryScalarOp};

    #[test]
    fn stream_unary() {
        const LOOP_BOUND: u64 = 16;

        // We will use seq length of 5 for now. So, conservatively keep the FIFO (Channel) size as 5.
        let chan_size = 2; // FIFO Depth

        let mut parent = Program::default();

        // Create Generator
        let (in_sender, in_receiver) = parent.bounded::<f64>(chan_size);
        let in_iter = || (0..LOOP_BOUND).map(|_i| 1_f64);
        let gen = GeneratorContext::new(in_iter, in_sender);

        // Create the Unary Block
        let (exp_sender, exp_receiver) = parent.bounded::<f64>(chan_size);
        let data = UnaryScalarData::<f64> {
            in_stream: in_receiver,
            out_list: vec![exp_sender],
            latency: 4,
            init_inverval: 1,
            loop_bound: LOOP_BOUND,
        };

        let stream_exp = UnaryScalarOp::new(data, UnaryOpType::Exp);

        // Create the Iterators for Checkers
        let out_iter = || (0..LOOP_BOUND).map(|_i| (1_f64).exp());

        let exp_checker = CheckerContext::new(out_iter, exp_receiver);

        parent.add_child(gen);
        parent.add_child(stream_exp);
        parent.add_child(exp_checker);
        parent.init();
        parent.run();
    }
}
