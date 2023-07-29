use dam_core::identifier::Identifier;
use dam_core::TimeManager;
use dam_macros::{cleanup, identifiable, time_managed};

use ndarray::{Array, ArrayBase, Dim, OwnedRepr};

use crate::{
    channel::{
        utils::{dequeue, enqueue, peek_next},
        ChannelElement, Receiver, Sender,
    },
    context::Context,
    types::{Cleanable, DAMType},
};

pub struct ReduceData<A: Clone> {
    pub in_stream: Receiver<ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>,
    pub out_stream: Sender<ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>,
    //pub latency: u64,       // pipeline depth -> We assume a single cycle
    pub init_inverval: u64, // initiation interval
    pub seq_len: u64,
}

impl<A: DAMType> Cleanable for ReduceData<A>
where
    ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>: DAMType,
{
    fn cleanup(&mut self) {
        self.in_stream.cleanup();
        self.out_stream.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct RowMax<A: Clone> {
    reduce_data: ReduceData<A>,
}

impl<A: DAMType> RowMax<A>
where
    RowMax<A>: Context,
    ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>: DAMType,
{
    pub fn new(reduce_data: ReduceData<A>) -> Self {
        let reduce = RowMax {
            reduce_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (reduce.reduce_data.in_stream).attach_receiver(&reduce);
        (reduce.reduce_data.out_stream).attach_sender(&reduce);

        reduce
    }
}

impl<A> Context for RowMax<A>
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
        for _i in 0..self.reduce_data.seq_len {
            let first_peek = peek_next(&mut self.time, &mut self.reduce_data.in_stream);
            match first_peek {
                Ok(first_elem) => {
                    let mut temp_res = first_elem.data;
                    for _i in 0..self.reduce_data.seq_len {
                        let in_deq = dequeue(&mut self.time, &mut self.reduce_data.in_stream);
                        match in_deq {
                            Ok(in_elem) => {
                                let in_data = in_elem.data;
                                let temp_res_1_d = temp_res.row(0);
                                let in_data_1_d = in_data.row(0);
                                let new_temp = Array::from_shape_vec(
                                    (1, in_data_1_d.len()),
                                    (0..in_data_1_d.len())
                                        .map(|i| {
                                            if temp_res_1_d[[i]] > in_data_1_d[[i]] {
                                                temp_res_1_d[[i]]
                                            } else {
                                                in_data_1_d[[i]]
                                            }
                                        })
                                        .collect(),
                                )
                                .unwrap();

                                temp_res = new_temp;
                                self.time.incr_cycles(self.reduce_data.init_inverval);
                            }
                            _ => {
                                panic!("Reached unhandled case");
                            }
                        }
                    }
                    let curr_time = self.time.tick();
                    enqueue(
                        &mut self.time,
                        &mut self.reduce_data.out_stream,
                        ChannelElement::new(curr_time, temp_res),
                    )
                    .unwrap();
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

#[time_managed]
#[identifiable]
pub struct RowSum<A: Clone> {
    reduce_data: ReduceData<A>,
}

impl<A: DAMType> RowSum<A>
where
    RowSum<A>: Context,
    ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>: DAMType,
{
    pub fn new(reduce_data: ReduceData<A>) -> Self {
        let reduce = RowSum {
            reduce_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (reduce.reduce_data.in_stream).attach_receiver(&reduce);
        (reduce.reduce_data.out_stream).attach_sender(&reduce);

        reduce
    }
}

impl<A> Context for RowSum<A>
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
        for _i in 0..self.reduce_data.seq_len {
            // outer loop
            let first_peek = dequeue(&mut self.time, &mut self.reduce_data.in_stream);
            match first_peek {
                Ok(first_elem) => {
                    let mut temp_res = first_elem.data;
                    self.time.incr_cycles(self.reduce_data.init_inverval);
                    for _i in 1..self.reduce_data.seq_len {
                        // inner loop
                        let in_deq = dequeue(&mut self.time, &mut self.reduce_data.in_stream);
                        match in_deq {
                            Ok(in_elem) => {
                                let new_temp = &temp_res + &in_elem.data;
                                temp_res = new_temp;
                                self.time.incr_cycles(self.reduce_data.init_inverval);
                            }
                            _ => {
                                panic!("Reached unhandled case");
                            }
                        }
                    }
                    let curr_time = self.time.tick();
                    enqueue(
                        &mut self.time,
                        &mut self.reduce_data.out_stream,
                        ChannelElement::new(curr_time, temp_res),
                    )
                    .unwrap();
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
    use ndarray::{array, Array, ArrayBase, Dim, OwnedRepr};

    use super::{ReduceData, RowMax, RowSum};

    #[test]
    fn test_reduce_array() {
        let mut temp_res = array![[1, 4, 3]];
        let in_data = array![[3, 2, 5]];
        let temp_res_1_d = temp_res.row(0);
        let in_data_1_d = in_data.row(0);
        temp_res = Array::from_shape_vec(
            (1, 3),
            (0..3)
                .map(|i| {
                    if temp_res_1_d[[i]] > in_data_1_d[[i]] {
                        temp_res_1_d[[i]]
                    } else {
                        in_data_1_d[[i]]
                    }
                })
                .collect(),
        )
        .unwrap();
        println!("{:?}", temp_res);
    }

    #[test]
    fn stream_reduce_max_test() {
        const HEAD_DIM: usize = 16;
        const SEQ_LEN: u64 = 5;
        const LATENCY: u64 = SEQ_LEN;
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
        let (out_sender, out_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>>(chan_size);

        // Create the Reduce block
        let data = ReduceData::<i32> {
            in_stream: in1_receiver,
            out_stream: out_sender,
            //latency: LATENCY,
            init_inverval: INIT_INTERVAL,
            seq_len: SEQ_LEN,
        };

        let stream_reduce_max = RowMax::new(data);

        // Create the Iterators for Generators
        let in1_iter = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|i| array![[i]]);

        // Create the Iterators for Checkers
        let out_iter = || (0..(SEQ_LEN_I32)).map(|i| array![[(i + 1) * SEQ_LEN_I32 - 1]]);

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
        const LATENCY: u64 = SEQ_LEN;
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
        let (out_sender, out_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>>(chan_size);

        // Create the Reduce block
        let data = ReduceData::<i32> {
            in_stream: in1_receiver,
            out_stream: out_sender,
            //latency: LATENCY,
            init_inverval: INIT_INTERVAL,
            seq_len: SEQ_LEN,
        };

        let stream_reduce_sum = RowSum::new(data);

        // Create the Iterators for Generators
        let in1_iter = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|_i| array![[1]]);

        // Create the Iterators for Checkers
        let out_iter = || (0..(SEQ_LEN_I32)).map(|_i| array![[SEQ_LEN_I32]]);

        let gen1 = GeneratorContext::new(in1_iter, in1_sender); // Q : [1,D] shaped vectors
        let stream_reduce_checker = CheckerContext::new(out_iter, out_receiver);

        parent.add_child(gen1);
        parent.add_child(stream_reduce_checker);
        parent.add_child(stream_reduce_sum);
        parent.init();
        parent.run();
    }
}
