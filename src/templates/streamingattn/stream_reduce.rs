use dam_core::identifier::Identifier;
use dam_core::TimeManager;
use dam_macros::{cleanup, identifiable, time_managed};

use std::cmp;

use crate::{
    channel::{
        utils::{dequeue, enqueue},
        ChannelElement, Receiver, Sender,
    },
    context::Context,
    types::{Cleanable, DAMType},
};

pub struct ReduceData<A: Clone> {
    // performs a reduction over a inner_loop_bound long vector
    // the computation is done in the scalar granularity
    pub in_stream: Receiver<A>, // operand: scalar (element of a 'inner_loop_bound' long vector)
    pub out_stream: Sender<A>,  // output -> scalar FIFO
    pub latency: u64,           // pipeline depth to do a computation on a scalar value
    pub init_inverval: u64,     // initiation interval
    pub inner_loop_bound: u64, // As this is a reduction, we need a inner loop bound to specify how many elements are reduce
    pub outer_loop_bound: u64,
}

impl<A: DAMType> Cleanable for ReduceData<A> {
    fn cleanup(&mut self) {
        self.in_stream.cleanup();
        self.out_stream.cleanup();
    }
}

pub enum ReduceOpType {
    Max,
    Sum,
}

#[time_managed]
#[identifiable]
pub struct ReduceOp<A: Clone> {
    reduce_data: ReduceData<A>,
    op: ReduceOpType,
}

impl<A: DAMType> ReduceOp<A>
where
    ReduceOp<A>: Context,
{
    pub fn new(reduce_data: ReduceData<A>, op: ReduceOpType) -> Self {
        let reduce = ReduceOp {
            reduce_data,
            op,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (reduce.reduce_data.in_stream).attach_receiver(&reduce);
        (reduce.reduce_data.out_stream).attach_sender(&reduce);

        reduce
    }
}

impl<A> Context for ReduceOp<A>
where
    A: DAMType + num::Num + std::cmp::Ord + Copy,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        for _i in 0..self.reduce_data.outer_loop_bound {
            let first_peek = dequeue(&mut self.time, &mut self.reduce_data.in_stream);
            match first_peek {
                Ok(first_elem) => {
                    let mut temp_res = first_elem.data;
                    self.time.incr_cycles(self.reduce_data.init_inverval);
                    for i in 1..self.reduce_data.inner_loop_bound {
                        let in_deq = dequeue(&mut self.time, &mut self.reduce_data.in_stream);
                        match in_deq {
                            Ok(in_elem) => {
                                let in_data = in_elem.data;
                                match self.op {
                                    ReduceOpType::Max => {
                                        temp_res = cmp::max(temp_res, in_data);
                                    }
                                    ReduceOpType::Sum => {
                                        temp_res = temp_res + in_data;
                                    }
                                }
                            }
                            _ => {
                                panic!("Reached unhandled case");
                            }
                        }
                        if i == self.reduce_data.inner_loop_bound - 1 {
                            let curr_time = self.time.tick();
                            enqueue(
                                &mut self.time,
                                &mut self.reduce_data.out_stream,
                                ChannelElement::new(curr_time + self.reduce_data.latency, temp_res),
                            )
                            .unwrap();
                        }
                        self.time.incr_cycles(self.reduce_data.init_inverval);
                    }
                }
                _ => {
                    panic!("Reached unhandled case");
                }
            }
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.reduce_data.cleanup();
        self.time.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        context::{checker_context::CheckerContext, generator_context::GeneratorContext},
        simulation::Program,
    };

    use super::{ReduceData, ReduceOp, ReduceOpType};

    #[test]
    fn stream_reduce_max_test() {
        const HEAD_DIM: usize = 16;
        const SEQ_LEN: u64 = 5;
        const LATENCY: u64 = 1;
        const INIT_INTERVAL: u64 = 1;

        // I32 types for generating data
        const SEQ_LEN_I32: i32 = 5;

        // We will use seq length of 5 for now. So, conservatively keep the FIFO (Channel) size as 5.
        let chan_size = 2; // FIFO Depth

        let mut parent = Program::default();

        // Create Channels
        let (in1_sender, in1_receiver) = parent.bounded::<i32>(chan_size);
        let (out_sender, out_receiver) = parent.bounded::<i32>(chan_size);

        // Create the Reduce block
        let data = ReduceData::<i32> {
            in_stream: in1_receiver,
            out_stream: out_sender,
            latency: LATENCY,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };

        let stream_reduce_max = ReduceOp::new(data, ReduceOpType::Max);

        // Create the Iterators for Generators
        let in1_iter = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32));

        // Create the Iterators for Checkers
        let out_iter = || (0..(SEQ_LEN_I32)).map(|i| (i + 1) * SEQ_LEN_I32 - 1);

        let gen1 = GeneratorContext::new(in1_iter, in1_sender); // Q : [1,D] shaped vectors
        let stream_reduce_checker = CheckerContext::new(out_iter, out_receiver);

        parent.add_child(gen1);
        parent.add_child(stream_reduce_checker);
        parent.add_child(stream_reduce_max);
        parent.init();
        parent.run();
    }

    #[test]
    fn stream_reduce_sum_test() {
        const HEAD_DIM: usize = 16;
        const SEQ_LEN: u64 = 5;
        const LATENCY: u64 = 1;
        const INIT_INTERVAL: u64 = 1;

        // I32 types for generating data
        const SEQ_LEN_I32: i32 = 5;

        // We will use seq length of 5 for now. So, conservatively keep the FIFO (Channel) size as 5.
        let chan_size = 2; // FIFO Depth

        let mut parent = Program::default();

        // Create Channels
        let (in1_sender, in1_receiver) = parent.bounded::<i32>(chan_size);
        let (out_sender, out_receiver) = parent.bounded::<i32>(chan_size);

        // Create the Reduce block
        let data = ReduceData::<i32> {
            in_stream: in1_receiver,
            out_stream: out_sender,
            latency: LATENCY,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };

        let stream_reduce_sum = ReduceOp::new(data, ReduceOpType::Sum);

        // Create the Iterators for Generators
        let in1_iter = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|_i| 1);

        // Create the Iterators for Checkers
        let out_iter = || (0..(SEQ_LEN_I32)).map(|_i| SEQ_LEN_I32);

        let gen1 = GeneratorContext::new(in1_iter, in1_sender); // Q : [1,D] shaped vectors
        let stream_reduce_checker = CheckerContext::new(out_iter, out_receiver);

        parent.add_child(gen1);
        parent.add_child(stream_reduce_checker);
        parent.add_child(stream_reduce_sum);
        parent.init();
        parent.run();
    }
}
