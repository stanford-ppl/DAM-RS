use dam_core::identifier::Identifier;
use dam_core::TimeManager;
use dam_macros::{cleanup, identifiable, time_managed};

use crate::{
    channel::{
        utils::{dequeue, enqueue, peek_next},
        ChannelElement, Receiver, Sender,
    },
    context::Context,
    types::{Cleanable, DAMType},
};

pub struct BinaryDataElemwise<A: Clone> {
    // Performs binary op on two scalars: A @ B (element-wise)
    pub in1_stream: Receiver<A>,
    pub in2_stream: Receiver<A>,
    pub out_stream: Sender<A>,
    pub latency: u64,       // pipeline depth
    pub init_inverval: u64, // initiation interval
    pub loop_bound: u64,    // As this is a element-wise op, we only need a single loop bound
}
pub struct BinaryDataOut1<A: Clone> {
    // Performs binary op on two scalars: A @ B (B reused for 'inner_loop_bound' As)
    pub in1_stream: Receiver<A>, // operand 1: A
    pub in2_stream: Receiver<A>, // operand 2: B
    pub out1_stream: Sender<A>,
    pub latency: u64,       // pipeline depth
    pub init_inverval: u64, // initiation interval
    pub inner_loop_bound: u64,
    pub outer_loop_bound: u64,
}

pub struct BinaryDataOut2<A: Clone> {
    // Performs binary op on two scalars: A @ B (B reused for 'inner_loop_bound' As)
    pub in1_stream: Receiver<A>, // operand 1: A
    pub in2_stream: Receiver<A>, // operand 2: B
    pub out1_stream: Sender<A>,
    pub out2_stream: Sender<A>,
    pub latency: u64,       // pipeline depth
    pub init_inverval: u64, // initiation interval
    pub inner_loop_bound: u64,
    pub outer_loop_bound: u64,
}

impl<A: DAMType> Cleanable for BinaryDataElemwise<A> {
    fn cleanup(&mut self) {
        self.in1_stream.cleanup();
        self.in2_stream.cleanup();
        self.out_stream.cleanup();
    }
}

impl<A: DAMType> Cleanable for BinaryDataOut1<A> {
    fn cleanup(&mut self) {
        self.in1_stream.cleanup();
        self.in2_stream.cleanup();
        self.out1_stream.cleanup();
    }
}

impl<A: DAMType> Cleanable for BinaryDataOut2<A> {
    fn cleanup(&mut self) {
        self.in1_stream.cleanup();
        self.in2_stream.cleanup();
        self.out1_stream.cleanup();
        self.out2_stream.cleanup();
    }
}

pub enum BinaryOpType {
    Add,
    Sub,
    Div,
    Mul,
}

#[time_managed]
#[identifiable]
pub struct BinaryOp<A: Clone> {
    binary_data: BinaryDataElemwise<A>,
    op: BinaryOpType,
}

impl<A: DAMType> BinaryOp<A>
where
    BinaryOp<A>: Context,
{
    pub fn new(binary_data: BinaryDataElemwise<A>, op: BinaryOpType) -> Self {
        let binary_op = BinaryOp {
            binary_data,
            op,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (binary_op.binary_data.in1_stream).attach_receiver(&binary_op);
        (binary_op.binary_data.in2_stream).attach_receiver(&binary_op);
        (binary_op.binary_data.out_stream).attach_sender(&binary_op);

        binary_op
    }
}

impl<A> Context for BinaryOp<A>
where
    A: DAMType + num::Num,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        for _i in 0..self.binary_data.loop_bound {
            let in1_deq = dequeue(&mut self.time, &mut self.binary_data.in1_stream);
            let in2_deq = dequeue(&mut self.time, &mut self.binary_data.in2_stream);

            match (in1_deq, in2_deq) {
                (Ok(in1), Ok(in2)) => {
                    let in1_data = in1.data;
                    let in2_data = in2.data;
                    let out_data: A;
                    match self.op {
                        BinaryOpType::Add => {
                            out_data = in1_data + in2_data;
                        }
                        BinaryOpType::Div => {
                            out_data = in1_data / in2_data;
                        }
                        BinaryOpType::Mul => {
                            out_data = in1_data * in2_data;
                        }
                        BinaryOpType::Sub => {
                            out_data = in1_data - in2_data;
                        }
                    }
                    let curr_time = self.time.tick();
                    enqueue(
                        &mut self.time,
                        &mut self.binary_data.out_stream,
                        ChannelElement::new(curr_time + self.binary_data.latency, out_data),
                    )
                    .unwrap();
                }
                (_, _) => {
                    panic!("Reached unhandled case");
                }
            }
            self.time.incr_cycles(self.binary_data.init_inverval);
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.binary_data.cleanup();
        self.time.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct BinaryOpOut1<A: Clone> {
    binary_data: BinaryDataOut1<A>,
    op: BinaryOpType,
}

impl<A: DAMType> BinaryOpOut1<A>
where
    BinaryOpOut1<A>: Context,
{
    pub fn new(binary_data: BinaryDataOut1<A>, op: BinaryOpType) -> Self {
        let binary_op = BinaryOpOut1 {
            binary_data,
            op,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (binary_op.binary_data.in1_stream).attach_receiver(&binary_op);
        (binary_op.binary_data.in2_stream).attach_receiver(&binary_op);
        (binary_op.binary_data.out1_stream).attach_sender(&binary_op);

        binary_op
    }
}

impl<A> Context for BinaryOpOut1<A>
where
    A: DAMType + num::Num,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        self.time.incr_cycles(4);
        for _i in 0..self.binary_data.outer_loop_bound {
            self.time.incr_cycles(4);
            let _ = peek_next(&mut self.time, &mut self.binary_data.in1_stream);
            let _ = peek_next(&mut self.time, &mut self.binary_data.in2_stream);
            let in1_deq = dequeue(&mut self.time, &mut self.binary_data.in1_stream);
            let in2_deq = dequeue(&mut self.time, &mut self.binary_data.in2_stream);

            match (in1_deq, in2_deq) {
                (Ok(in1), Ok(in2)) => {
                    let in1_data = in1.data;
                    let in2_data = in2.data;
                    let out_data: A;
                    match self.op {
                        BinaryOpType::Add => {
                            out_data = in1_data + in2_data.clone();
                        }
                        BinaryOpType::Div => {
                            out_data = in1_data / in2_data.clone();
                        }
                        BinaryOpType::Mul => {
                            out_data = in1_data * in2_data.clone();
                        }
                        BinaryOpType::Sub => {
                            out_data = in1_data - in2_data.clone();
                        }
                    }
                    let curr_time = self.time.tick();
                    enqueue(
                        &mut self.time,
                        &mut self.binary_data.out1_stream,
                        ChannelElement::new(curr_time + self.binary_data.latency, out_data.clone()),
                    )
                    .unwrap();

                    self.time.incr_cycles(self.binary_data.init_inverval);

                    for _i in 1..self.binary_data.inner_loop_bound {
                        let in1_deq = dequeue(&mut self.time, &mut self.binary_data.in1_stream);

                        match in1_deq {
                            Ok(in1) => {
                                let in1_data = in1.data;
                                let out_data: A;
                                match self.op {
                                    BinaryOpType::Add => {
                                        out_data = in1_data + in2_data.clone();
                                    }
                                    BinaryOpType::Div => {
                                        out_data = in1_data / in2_data.clone();
                                    }
                                    BinaryOpType::Mul => {
                                        out_data = in1_data * in2_data.clone();
                                    }
                                    BinaryOpType::Sub => {
                                        out_data = in1_data - in2_data.clone();
                                    }
                                }
                                let curr_time = self.time.tick();
                                enqueue(
                                    &mut self.time,
                                    &mut self.binary_data.out1_stream,
                                    ChannelElement::new(
                                        curr_time + self.binary_data.latency,
                                        out_data.clone(),
                                    ),
                                )
                                .unwrap();

                                self.time.incr_cycles(self.binary_data.init_inverval);
                            }
                            _ => {
                                panic!("Reached unhandled case");
                            }
                        }
                    }
                }
                (_, _) => {
                    panic!("Reached unhandled case");
                }
            }
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.binary_data.cleanup();
        self.time.cleanup();

        // let curr_time = self.time.tick();
        // println!("Binary");
        // dbg!(curr_time);
    }
}

#[time_managed]
#[identifiable]
pub struct BinaryOpOut2<A: Clone> {
    binary_data: BinaryDataOut2<A>,
    op: BinaryOpType,
}

impl<A: DAMType> BinaryOpOut2<A>
where
    BinaryOpOut2<A>: Context,
{
    pub fn new(binary_data: BinaryDataOut2<A>, op: BinaryOpType) -> Self {
        let binary_op = BinaryOpOut2 {
            binary_data,
            op,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (binary_op.binary_data.in1_stream).attach_receiver(&binary_op);
        (binary_op.binary_data.in2_stream).attach_receiver(&binary_op);
        (binary_op.binary_data.out1_stream).attach_sender(&binary_op);
        (binary_op.binary_data.out2_stream).attach_sender(&binary_op);

        binary_op
    }
}

impl<A> Context for BinaryOpOut2<A>
where
    A: DAMType + num::Num,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        for _i in 0..self.binary_data.outer_loop_bound {
            let in1_deq = dequeue(&mut self.time, &mut self.binary_data.in1_stream);
            let in2_deq = dequeue(&mut self.time, &mut self.binary_data.in2_stream);

            match (in1_deq, in2_deq) {
                (Ok(in1), Ok(in2)) => {
                    let in1_data = in1.data;
                    let in2_data = in2.data;
                    let out_data: A;
                    match self.op {
                        BinaryOpType::Add => {
                            out_data = in1_data + in2_data.clone();
                        }
                        BinaryOpType::Div => {
                            out_data = in1_data / in2_data.clone();
                        }
                        BinaryOpType::Mul => {
                            out_data = in1_data * in2_data.clone();
                        }
                        BinaryOpType::Sub => {
                            out_data = in1_data - in2_data.clone();
                        }
                    }
                    let curr_time = self.time.tick();
                    enqueue(
                        &mut self.time,
                        &mut self.binary_data.out1_stream,
                        ChannelElement::new(curr_time + self.binary_data.latency, out_data.clone()),
                    )
                    .unwrap();
                    enqueue(
                        &mut self.time,
                        &mut self.binary_data.out2_stream,
                        ChannelElement::new(curr_time + self.binary_data.latency, out_data),
                    )
                    .unwrap();

                    self.time.incr_cycles(self.binary_data.init_inverval);

                    for _i in 1..self.binary_data.inner_loop_bound {
                        let in1_deq = dequeue(&mut self.time, &mut self.binary_data.in1_stream);

                        match in1_deq {
                            Ok(in1) => {
                                let in1_data = in1.data;
                                let out_data: A;
                                match self.op {
                                    BinaryOpType::Add => {
                                        out_data = in1_data + in2_data.clone();
                                    }
                                    BinaryOpType::Div => {
                                        out_data = in1_data / in2_data.clone();
                                    }
                                    BinaryOpType::Mul => {
                                        out_data = in1_data * in2_data.clone();
                                    }
                                    BinaryOpType::Sub => {
                                        out_data = in1_data - in2_data.clone();
                                    }
                                }
                                let curr_time = self.time.tick();
                                enqueue(
                                    &mut self.time,
                                    &mut self.binary_data.out1_stream,
                                    ChannelElement::new(
                                        curr_time + self.binary_data.latency,
                                        out_data.clone(),
                                    ),
                                )
                                .unwrap();
                                enqueue(
                                    &mut self.time,
                                    &mut self.binary_data.out2_stream,
                                    ChannelElement::new(
                                        curr_time + self.binary_data.latency,
                                        out_data,
                                    ),
                                )
                                .unwrap();

                                self.time.incr_cycles(self.binary_data.init_inverval);
                            }
                            _ => {
                                panic!("Reached unhandled case");
                            }
                        }
                    }
                }
                (_, _) => {
                    panic!("Reached unhandled case");
                }
            }
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.binary_data.cleanup();
        self.time.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        context::{checker_context::CheckerContext, generator_context::GeneratorContext},
        simulation::Program,
    };

    use std::iter;

    use super::{
        BinaryDataElemwise, BinaryDataOut1, BinaryDataOut2, BinaryOp, BinaryOpOut1, BinaryOpOut2,
        BinaryOpType,
    };

    #[test]
    fn stream_binary_sub_test() {
        const HEAD_DIM: usize = 16;
        const SEQ_LEN: u64 = 5;
        const INIT_INTERVAL: u64 = 1;
        const LATENCY: u64 = 1;

        // I32 types for generating data
        const SEQ_LEN_USIZE: usize = 5;
        const SEQ_LEN_I32: i32 = 5;
        const HEAD_DIM_I32: i32 = 16;

        // We will use seq length of 5 for now. So, conservatively keep the FIFO (Channel) size as 5.
        let chan_size = 2; // FIFO Depth

        let mut parent = Program::default();

        // Create Channels
        let (in1_sender, in1_receiver) = parent.bounded::<i32>(chan_size);
        let (in2_sender, in2_receiver) = parent.bounded::<i32>(chan_size);
        let (out_sender, out_receiver) = parent.bounded::<i32>(chan_size);

        // Create the Reduce block
        let data = BinaryDataElemwise::<i32> {
            in1_stream: in1_receiver,
            in2_stream: in2_receiver,
            out_stream: out_sender,
            latency: LATENCY,
            init_inverval: INIT_INTERVAL,
            loop_bound: SEQ_LEN * SEQ_LEN,
        };

        let stream_binary_sub = BinaryOp::new(data, BinaryOpType::Sub);

        // Create the Iterators for Generators
        let in1_iter = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32));
        let in2_iter = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32));

        // Create the Iterators for Checkers
        let out_iter = || iter::repeat(0).take(SEQ_LEN_USIZE * SEQ_LEN_USIZE);

        let gen1 = GeneratorContext::new(in1_iter, in1_sender); // Q : [1,D] shaped vectors
        let gen2 = GeneratorContext::new(in2_iter, in2_sender); // Q : [1,D] shaped vectors
        let stream_binary_checker = CheckerContext::new(out_iter, out_receiver);

        parent.add_child(gen1);
        parent.add_child(gen2);
        parent.add_child(stream_binary_checker);
        parent.add_child(stream_binary_sub);
        parent.init();
        parent.run();
    }

    #[test]
    fn stream_binary_div_test() {
        const HEAD_DIM: usize = 16;
        const SEQ_LEN: u64 = 5;
        const INIT_INTERVAL: u64 = 1;
        const LATENCY: u64 = 1;

        // I32 types for generating data
        const SEQ_LEN_USIZE: usize = 5;
        const SEQ_LEN_I32: i32 = 5;
        const HEAD_DIM_I32: i32 = 16;

        // We will use seq length of 5 for now. So, conservatively keep the FIFO (Channel) size as 5.
        let chan_size = 2; // FIFO Depth

        let mut parent = Program::default();

        // Create Channels
        let (in1_sender, in1_receiver) = parent.bounded::<i32>(chan_size);
        let (in2_sender, in2_receiver) = parent.bounded::<i32>(chan_size);
        let (out_sender, out_receiver) = parent.bounded::<i32>(chan_size);

        // Create the Reduce block
        let data = BinaryDataElemwise::<i32> {
            in1_stream: in1_receiver,
            in2_stream: in2_receiver,
            out_stream: out_sender,
            latency: LATENCY,
            init_inverval: INIT_INTERVAL,
            loop_bound: SEQ_LEN * SEQ_LEN,
        };

        let stream_binary_div = BinaryOp::new(data, BinaryOpType::Div);

        // Create the Iterators for Generators
        let in1_iter = || (1..(SEQ_LEN_I32 * SEQ_LEN_I32 + 1));
        let in2_iter = || (1..(SEQ_LEN_I32 * SEQ_LEN_I32 + 1));

        // Create the Iterators for Checkers
        let out_iter = || iter::repeat(1).take(SEQ_LEN_USIZE * SEQ_LEN_USIZE);

        let gen1 = GeneratorContext::new(in1_iter, in1_sender); // Q : [1,D] shaped vectors
        let gen2 = GeneratorContext::new(in2_iter, in2_sender); // Q : [1,D] shaped vectors
        let stream_binary_checker = CheckerContext::new(out_iter, out_receiver);

        parent.add_child(gen1);
        parent.add_child(gen2);
        parent.add_child(stream_binary_checker);
        parent.add_child(stream_binary_div);
        parent.init();
        parent.run();
    }

    #[test]
    fn stream_binary_out1_div_test() {
        const HEAD_DIM: usize = 16;
        const SEQ_LEN: u64 = 5;
        const INIT_INTERVAL: u64 = 1;
        const LATENCY: u64 = 1;

        // I32 types for generating data
        const SEQ_LEN_USIZE: usize = 5;
        const SEQ_LEN_I32: i32 = 5;
        const HEAD_DIM_I32: i32 = 16;

        // We will use seq length of 5 for now. So, conservatively keep the FIFO (Channel) size as 5.
        let chan_size = 2; // FIFO Depth

        let mut parent = Program::default();

        // Create Channels
        let (in1_sender, in1_receiver) = parent.bounded::<i32>(chan_size);
        let (in2_sender, in2_receiver) = parent.bounded::<i32>(chan_size);
        let (out1_sender, out1_receiver) = parent.bounded::<i32>(chan_size);

        // Create the Reduce block
        let data = BinaryDataOut1::<i32> {
            in1_stream: in1_receiver,
            in2_stream: in2_receiver,
            out1_stream: out1_sender,
            latency: LATENCY,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };

        let stream_binary_div = BinaryOpOut1::new(data, BinaryOpType::Div);

        // Create the Iterators for Generators
        let in1_iter = || (1..(SEQ_LEN_I32 * SEQ_LEN_I32 + 1));
        let in2_iter = || (0..SEQ_LEN_I32).map(|_x| 1);

        // Create the Iterators for Checkers
        let out1_iter = || (1..(SEQ_LEN_I32 * SEQ_LEN_I32 + 1));

        let gen1 = GeneratorContext::new(in1_iter, in1_sender); // Q : [1,D] shaped vectors
        let gen2 = GeneratorContext::new(in2_iter, in2_sender); // Q : [1,D] shaped vectors
        let stream_binary_checker1 = CheckerContext::new(out1_iter, out1_receiver);

        parent.add_child(gen1);
        parent.add_child(gen2);
        parent.add_child(stream_binary_checker1);
        parent.add_child(stream_binary_div);
        parent.init();
        parent.run();
    }

    #[test]
    fn stream_binary_out2_div_test() {
        const HEAD_DIM: usize = 16;
        const SEQ_LEN: u64 = 5;
        const INIT_INTERVAL: u64 = 1;
        const LATENCY: u64 = 1;

        // I32 types for generating data
        const SEQ_LEN_USIZE: usize = 5;
        const SEQ_LEN_I32: i32 = 5;
        const HEAD_DIM_I32: i32 = 16;

        // We will use seq length of 5 for now. So, conservatively keep the FIFO (Channel) size as 5.
        let chan_size = 2; // FIFO Depth

        let mut parent = Program::default();

        // Create Channels
        let (in1_sender, in1_receiver) = parent.bounded::<i32>(chan_size);
        let (in2_sender, in2_receiver) = parent.bounded::<i32>(chan_size);
        let (out1_sender, out1_receiver) = parent.bounded::<i32>(chan_size);
        let (out2_sender, out2_receiver) = parent.bounded::<i32>(chan_size);

        // Create the Reduce block
        let data = BinaryDataOut2::<i32> {
            in1_stream: in1_receiver,
            in2_stream: in2_receiver,
            out1_stream: out1_sender,
            out2_stream: out2_sender,
            latency: LATENCY,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };

        let stream_binary_div = BinaryOpOut2::new(data, BinaryOpType::Div);

        // Create the Iterators for Generators
        let in1_iter = || (1..(SEQ_LEN_I32 * SEQ_LEN_I32 + 1));
        let in2_iter = || (0..SEQ_LEN_I32).map(|_x| 1);

        // Create the Iterators for Checkers
        let out1_iter = || (1..(SEQ_LEN_I32 * SEQ_LEN_I32 + 1));
        let out2_iter = || (1..(SEQ_LEN_I32 * SEQ_LEN_I32 + 1));

        let gen1 = GeneratorContext::new(in1_iter, in1_sender); // Q : [1,D] shaped vectors
        let gen2 = GeneratorContext::new(in2_iter, in2_sender); // Q : [1,D] shaped vectors
        let stream_binary_checker1 = CheckerContext::new(out1_iter, out1_receiver);
        let stream_binary_checker2 = CheckerContext::new(out2_iter, out2_receiver);

        parent.add_child(gen1);
        parent.add_child(gen2);
        parent.add_child(stream_binary_checker1);
        parent.add_child(stream_binary_checker2);
        parent.add_child(stream_binary_div);
        parent.init();
        parent.run();
    }
}
