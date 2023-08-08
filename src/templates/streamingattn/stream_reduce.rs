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

pub struct ReduceData2<A: Clone> {
    // performs a reduction over a inner_loop_bound long vector
    // the computation is done in the scalar granularity
    pub in_stream: Receiver<A>, // operand: scalar (element of a 'inner_loop_bound' long vector)
    pub new_max: Sender<A>,     // output -> scalar FIFO
    pub old_new_diff: Sender<A>, // output -> scalar FIFO
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

impl<A: DAMType> Cleanable for ReduceData2<A> {
    fn cleanup(&mut self) {
        self.in_stream.cleanup();
        self.new_max.cleanup();
        self.old_new_diff.cleanup();
    }
}

pub enum ReduceOpType {
    Max,
    Sum,
}

pub trait MinMax {
    fn get_max(self, rhs: Self) -> Self;
    fn get_min_val() -> Self;
    fn get_zero() -> Self;
}
impl MinMax for u8 {
    fn get_max(self, rhs: u8) -> u8 {
        self.max(rhs)
    }

    fn get_min_val() -> u8 {
        u8::MIN
    }

    fn get_zero() -> u8 {
        0u8
    }
}
impl MinMax for u16 {
    fn get_max(self, rhs: u16) -> u16 {
        self.max(rhs)
    }
    fn get_min_val() -> u16 {
        u16::MIN
    }
    fn get_zero() -> u16 {
        0u16
    }
}
impl MinMax for u32 {
    fn get_max(self, rhs: u32) -> u32 {
        self.max(rhs)
    }
    fn get_min_val() -> u32 {
        u32::MIN
    }
    fn get_zero() -> u32 {
        0u32
    }
}
impl MinMax for u64 {
    fn get_max(self, rhs: u64) -> u64 {
        self.max(rhs)
    }
    fn get_min_val() -> u64 {
        u64::MIN
    }
    fn get_zero() -> u64 {
        0u64
    }
}
impl MinMax for i8 {
    fn get_max(self, rhs: i8) -> i8 {
        self.max(rhs)
    }
    fn get_min_val() -> i8 {
        i8::MIN
    }
    fn get_zero() -> i8 {
        0i8
    }
}
impl MinMax for i16 {
    fn get_max(self, rhs: i16) -> i16 {
        self.max(rhs)
    }
    fn get_min_val() -> i16 {
        i16::MIN
    }
    fn get_zero() -> i16 {
        0i16
    }
}
impl MinMax for i32 {
    fn get_max(self, rhs: i32) -> i32 {
        self.max(rhs)
    }
    fn get_min_val() -> i32 {
        i32::MIN
    }
    fn get_zero() -> i32 {
        0i32
    }
}
impl MinMax for i64 {
    fn get_max(self, rhs: i64) -> i64 {
        self.max(rhs)
    }
    fn get_min_val() -> i64 {
        i64::MIN
    }
    fn get_zero() -> i64 {
        0i64
    }
}
impl MinMax for f32 {
    fn get_max(self, rhs: f32) -> f32 {
        self.max(rhs)
    }
    fn get_min_val() -> f32 {
        f32::MIN
    }
    fn get_zero() -> f32 {
        0_f32
    }
}
impl MinMax for f64 {
    fn get_max(self, rhs: f64) -> f64 {
        self.max(rhs)
    }
    fn get_min_val() -> f64 {
        f64::MIN
    }
    fn get_zero() -> f64 {
        0_f64
    }
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
    A: DAMType + num::Num + MinMax + Copy,
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
                                        temp_res = temp_res.get_max(in_data);
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
        // let curr_time = self.time.tick();
        // println!("Reduce Op");
        // dbg!(curr_time);
    }
}

#[time_managed]
#[identifiable]
pub struct IncrReduceOp<A: Clone> {
    reduce_data: ReduceData2<A>,
    op: ReduceOpType,
}

impl<A: DAMType> IncrReduceOp<A>
where
    IncrReduceOp<A>: Context,
{
    pub fn new(reduce_data: ReduceData2<A>, op: ReduceOpType) -> Self {
        let reduce = IncrReduceOp {
            reduce_data,
            op,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (reduce.reduce_data.in_stream).attach_receiver(&reduce);
        (reduce.reduce_data.new_max).attach_sender(&reduce);
        (reduce.reduce_data.old_new_diff).attach_sender(&reduce);

        reduce
    }
}

impl<A> Context for IncrReduceOp<A>
where
    A: DAMType + num::Num + MinMax + Copy,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        for _i in 0..self.reduce_data.outer_loop_bound {
            let first_peek = dequeue(&mut self.time, &mut self.reduce_data.in_stream);
            match first_peek {
                Ok(first_elem) => {
                    // First Iteration
                    let mut temp_res = first_elem.data;
                    let curr_time = self.time.tick();
                    enqueue(
                        &mut self.time,
                        &mut self.reduce_data.new_max,
                        ChannelElement::new(curr_time + self.reduce_data.latency, temp_res),
                    )
                    .unwrap();

                    enqueue(
                        &mut self.time,
                        &mut self.reduce_data.old_new_diff,
                        ChannelElement::new(
                            curr_time + self.reduce_data.latency,
                            temp_res - temp_res,
                        ),
                    )
                    .unwrap();

                    self.time.incr_cycles(self.reduce_data.init_inverval);

                    // From the second iteration
                    for _i in 1..self.reduce_data.inner_loop_bound {
                        let old_new_diff;
                        let in_deq = dequeue(&mut self.time, &mut self.reduce_data.in_stream);
                        match in_deq {
                            Ok(in_elem) => {
                                let in_data = in_elem.data;
                                match self.op {
                                    ReduceOpType::Max => {
                                        let new_max = temp_res.get_max(in_data);
                                        old_new_diff = temp_res - new_max;
                                        temp_res = new_max;
                                    }
                                    ReduceOpType::Sum => {
                                        temp_res = temp_res + in_data;
                                        old_new_diff = in_data;
                                    }
                                }
                            }
                            _ => {
                                panic!("Reached unhandled case");
                            }
                        }

                        let curr_time = self.time.tick();
                        enqueue(
                            &mut self.time,
                            &mut self.reduce_data.new_max,
                            ChannelElement::new(curr_time + self.reduce_data.latency, temp_res),
                        )
                        .unwrap();

                        enqueue(
                            &mut self.time,
                            &mut self.reduce_data.old_new_diff,
                            ChannelElement::new(curr_time + self.reduce_data.latency, old_new_diff),
                        )
                        .unwrap();

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

    use super::{IncrReduceOp, ReduceData, ReduceData2, ReduceOp, ReduceOpType};

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

    #[test]
    fn stream_incrmtl_reduce_max_test() {
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
        let (out1_sender, out1_receiver) = parent.bounded::<i32>(chan_size);
        let (out2_sender, out2_receiver) = parent.bounded::<i32>(chan_size);

        // Create the Reduce block
        let data = ReduceData2::<i32> {
            in_stream: in1_receiver,
            new_max: out1_sender,
            old_new_diff: out2_sender,
            latency: LATENCY + 1,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };

        let stream_reduce_max = IncrReduceOp::new(data, ReduceOpType::Max);

        // Create the Iterators for Generators
        let in1_iter = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32));

        // Create the Iterators for Checkers
        let out1_iter = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32));
        let out2_iter =
            || (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|x| if x % SEQ_LEN_I32 == 0 { 0 } else { -1 });

        let gen1 = GeneratorContext::new(in1_iter, in1_sender); // Q : [1,D] shaped vectors
        let stream_reduce_checker1 = CheckerContext::new(out1_iter, out1_receiver);
        let stream_reduce_checker2 = CheckerContext::new(out2_iter, out2_receiver);

        parent.add_child(gen1);
        parent.add_child(stream_reduce_checker1);
        parent.add_child(stream_reduce_checker2);
        parent.add_child(stream_reduce_max);
        parent.init();
        parent.run();
    }
}
