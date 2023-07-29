use dam_core::identifier::Identifier;
use dam_core::TimeManager;
use dam_macros::{cleanup, identifiable, time_managed};

use ndarray::{ArrayBase, Dim, OwnedRepr};

use crate::{
    channel::{
        utils::{dequeue, enqueue},
        ChannelElement, Receiver, Sender,
    },
    context::Context,
    types::{Cleanable, DAMType},
};

pub struct BinaryData<A: Clone> {
    pub in1_stream: Receiver<ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>,
    pub in2_stream: Receiver<ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>,
    pub out_stream: Sender<ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>,
    //pub latency: u64,       // pipeline depth -> We assume a single cycle
    pub init_inverval: u64, // initiation interval
    pub loop_bound: u64,
}

impl<A: DAMType> Cleanable for BinaryData<A>
where
    ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>: DAMType,
{
    fn cleanup(&mut self) {
        self.in1_stream.cleanup();
        self.in2_stream.cleanup();
        self.out_stream.cleanup();
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
    binary_data: BinaryData<A>,
    op: BinaryOpType,
}

impl<A: DAMType> BinaryOp<A>
where
    BinaryOp<A>: Context,
    ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>: DAMType,
{
    pub fn new(binary_data: BinaryData<A>, op: BinaryOpType) -> Self {
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
    ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>: DAMType
        //+ std::ops::Mul<Output = ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>
        + ndarray::linalg::Dot<
            ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>,
            Output = ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>,
        >,
    A: DAMType + num::Num + std::cmp::PartialOrd + Copy,
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
                    let out_data: ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>;
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
                        ChannelElement::new(curr_time + 1, out_data),
                    )
                    .unwrap();
                }
                (_, _) => {
                    panic!("Reached unhandled case");
                }
            }
            self.time.incr_cycles(1);
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
    use ndarray::{array, ArrayBase, Dim, OwnedRepr};

    use super::{BinaryData, BinaryOp, BinaryOpType};

    #[test]
    fn stream_binary_sub_test() {
        const HEAD_DIM: usize = 16;
        const SEQ_LEN: u64 = 5;
        const INIT_INTERVAL: u64 = 1;

        // I32 types for generating data
        const SEQ_LEN_I32: i32 = 5;
        const HEAD_DIM_I32: i32 = 16;

        // We will use seq length of 5 for now. So, conservatively keep the FIFO (Channel) size as 5.
        let chan_size = 2; // FIFO Depth

        let mut parent = Program::default();

        // Create Channels
        let (in1_sender, in1_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>>(chan_size);
        let (in2_sender, in2_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>>(chan_size);
        let (out_sender, out_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>>(chan_size);

        // Create the Reduce block
        let data = BinaryData::<i32> {
            in1_stream: in1_receiver,
            in2_stream: in2_receiver,
            out_stream: out_sender,
            init_inverval: INIT_INTERVAL,
            loop_bound: SEQ_LEN * SEQ_LEN,
        };

        let stream_binary_sub = BinaryOp::new(data, BinaryOpType::Sub);

        // Create the Iterators for Generators
        let in1_iter = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|i| array![[i]]);
        let in2_iter = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|i| array![[i]]);

        // Create the Iterators for Checkers
        let out_iter = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|_i| array![[0]]);

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

        // I32 types for generating data
        const SEQ_LEN_I32: i32 = 5;
        const HEAD_DIM_I32: i32 = 16;

        // We will use seq length of 5 for now. So, conservatively keep the FIFO (Channel) size as 5.
        let chan_size = 2; // FIFO Depth

        let mut parent = Program::default();

        // Create Channels
        let (in1_sender, in1_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>>(chan_size);
        let (in2_sender, in2_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>>(chan_size);
        let (out_sender, out_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>>(chan_size);

        // Create the Reduce block
        let data = BinaryData::<i32> {
            in1_stream: in1_receiver,
            in2_stream: in2_receiver,
            out_stream: out_sender,
            init_inverval: INIT_INTERVAL,
            loop_bound: SEQ_LEN * SEQ_LEN,
        };

        let stream_binary_div = BinaryOp::new(data, BinaryOpType::Div);

        // Create the Iterators for Generators
        let in1_iter = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|i| array![[i + 1]]);
        let in2_iter = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|i| array![[i + 1]]);

        // Create the Iterators for Checkers
        let out_iter = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|_i| array![[1]]);

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

}
