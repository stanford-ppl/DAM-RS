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

pub struct MatmulData<A: Clone> {
    // performs a outer product of two matrices
    // the computation is done in (scalar * vector) granularity and is accumulated in a vector granularity
    // reduce over 'inner_loop_bound' partial sums
    // repeat this reduction for 'outer_loop_bound' times
    pub in_scalar_stream: Receiver<A>, // operand 1: scalar (element of a 'inner_loop_bound' long vector)
    pub in_vec_stream: Receiver<ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>>, // operand 2: vector
    pub out_stream: Sender<ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>>, // output -> vector FIFO
    pub latency: u64, // pipeline depth to perform ((scalar * vector) -> accumulate)
    pub init_inverval: u64, // initiation interval
    pub inner_loop_bound: u64,
    pub outer_loop_bound: u64,
}

impl<A: DAMType> Cleanable for MatmulData<A>
where
    ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>: DAMType,
{
    fn cleanup(&mut self) {
        self.in_scalar_stream.cleanup();
        self.in_vec_stream.cleanup();
        self.out_stream.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct MatmulOuter<A: Clone> {
    matmul_data: MatmulData<A>,
}

impl<A: DAMType> MatmulOuter<A>
where
    MatmulOuter<A>: Context,
    ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>: DAMType,
{
    pub fn new(matmul_data: MatmulData<A>) -> Self {
        let matmul_outer = MatmulOuter {
            matmul_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (matmul_outer.matmul_data.in_scalar_stream).attach_receiver(&matmul_outer);
        (matmul_outer.matmul_data.in_vec_stream).attach_receiver(&matmul_outer);
        (matmul_outer.matmul_data.out_stream).attach_sender(&matmul_outer);

        matmul_outer
    }
}

impl<A> Context for MatmulOuter<A>
where
    ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>: DAMType,
    A: DAMType + num::Num + Copy,
{
    fn init(&mut self) {}
    fn run(&mut self) -> () {
        for _i in 0..self.matmul_data.outer_loop_bound {
            let first_scalar_deq = dequeue(&mut self.time, &mut self.matmul_data.in_scalar_stream);
            let first_vector_deq = dequeue(&mut self.time, &mut self.matmul_data.in_vec_stream);

            match (first_scalar_deq, first_vector_deq) {
                (Ok(first_scalar_elem), Ok(first_vec_elem)) => {
                    let first_scalar_data: A = first_scalar_elem.data;
                    let first_vec_data = first_vec_elem.data;
                    let mut accum_sum = first_vec_data.map(|x| -> A { first_scalar_data * (*x) });

                    self.time.incr_cycles(self.matmul_data.init_inverval);

                    for i in 1..self.matmul_data.inner_loop_bound {
                        let in_scalar_deq =
                            dequeue(&mut self.time, &mut self.matmul_data.in_scalar_stream);
                        let in_vector_deq =
                            dequeue(&mut self.time, &mut self.matmul_data.in_vec_stream);

                        match (in_scalar_deq, in_vector_deq) {
                            (Ok(in_scalar_elem), Ok(in_vector_elem)) => {
                                let in_scalar_data = in_scalar_elem.data;
                                let in_vector_data = in_vector_elem.data;
                                let curr_val =
                                    in_vector_data.map(|x| -> A { in_scalar_data * (*x) });
                                accum_sum = accum_sum + curr_val;
                            }
                            (_, _) => {
                                panic!("Reached unhandled case");
                            }
                        }
                        if i == self.matmul_data.inner_loop_bound - 1 {
                            let curr_time = self.time.tick();
                            enqueue(
                                &mut self.time,
                                &mut self.matmul_data.out_stream,
                                ChannelElement::new(
                                    curr_time + self.matmul_data.latency,
                                    accum_sum.clone(),
                                ),
                            )
                            .unwrap();
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
        self.matmul_data.cleanup();
        self.time.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::{MatmulData, MatmulOuter};
    use crate::{
        context::{checker_context::CheckerContext, generator_context::GeneratorContext},
        simulation::Program,
    };
    use ndarray::{Array, ArrayBase, Dim, OwnedRepr};

    #[test]
    fn stream_outer_product_test() {
        const LATENCY: u64 = 1;
        const INIT_INTERVAL: u64 = 1;
        const SEQ_LEN: u64 = 5;

        const HEAD_DIM: usize = 16;
        const SEQ_LEN_I32: i32 = 5;

        let chan_size = 2; // FIFO Depth

        let mut parent = Program::default();

        // Create Channels
        let (in1_sender, in1_receiver) = parent.bounded::<i32>(chan_size);
        let (in2_sender, in2_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 1]>>>(chan_size);
        let (out_sender, out_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 1]>>>(chan_size);

        let data = MatmulData::<i32> {
            in_scalar_stream: in1_receiver,
            in_vec_stream: in2_receiver,
            out_stream: out_sender,
            latency: LATENCY,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };

        let stream_outer_product = MatmulOuter::new(data);

        // Create the Iterators for Generators
        let in1_iter = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|i| i % SEQ_LEN_I32);
        let in2_iter = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|_i| Array::from_elem(HEAD_DIM, 1));

        // Create the Iterators for Checkers
        let out_iter = || {
            (0..(SEQ_LEN_I32))
                .map(|_i| Array::from_elem(HEAD_DIM, SEQ_LEN_I32 * (SEQ_LEN_I32 - 1) / 2))
        };

        let gen1 = GeneratorContext::new(in1_iter, in1_sender); // Q : [1,D] shaped vectors
        let gen2 = GeneratorContext::new(in2_iter, in2_sender); // Q : [1,D] shaped vectors
        let stream_outer_prod_checker = CheckerContext::new(out_iter, out_receiver);

        parent.add_child(gen1);
        parent.add_child(gen2);
        parent.add_child(stream_outer_prod_checker);
        parent.add_child(stream_outer_product);
        parent.init();
        parent.run();
    }
}
