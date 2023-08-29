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

use ndarray::{linalg, Array, ArrayBase, Dim, OwnedRepr};

pub struct MatMulWData<A: Clone> {
    pub in_stream: Receiver<ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>>,
    pub weight_stream: Receiver<ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>>, // connect to generator context & later connect that into DRAM
    // Like the way people do in FPGAs we can pre-load the weitghts to registers in advance and run.
    // However, here we still count the cycles it takes to inialize the vectors.
    pub out_stream: Sender<ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>>,
    pub in_size: usize,
    pub systolic_h: usize,
    pub systolic_w: usize,
}

impl<A: Clone> Cleanable for MatMulWData<A>
where
    ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>: DAMType,
{
    fn cleanup(&mut self) {
        self.in_stream.cleanup();
        self.weight_stream.cleanup();
        self.out_stream.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct MatMulW<A: Clone> {
    matmulw_data: MatMulWData<A>,
}

impl<A: Clone> MatMulW<A>
where
    MatMulW<A>: Context,
    ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>: DAMType,
{
    pub fn new(matmulw_data: MatMulWData<A>) -> Self {
        let matmul_w = MatMulW {
            matmulw_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (matmul_w.matmulw_data.in_stream).attach_receiver(&matmul_w);
        (matmul_w.matmulw_data.weight_stream).attach_receiver(&matmul_w);
        (matmul_w.matmulw_data.out_stream).attach_sender(&matmul_w);

        matmul_w
    }
}

impl<A> Context for MatMulW<A>
where
    ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>:
        DAMType + linalg::Dot<ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>, Output = A>,
    A: Clone + num::Num,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        // Initialize weight registers in each PE
        let mut weight_regs = Vec::new();
        for _i in 0..self.matmulw_data.systolic_w {
            let weight_deq = dequeue(&mut self.time, &mut self.matmulw_data.weight_stream);

            match weight_deq {
                Ok(weight_vec) => {
                    weight_regs.push(weight_vec.data);
                    self.time.incr_cycles(1);
                }
                _ => {
                    panic!("Reached unhandled case");
                }
            }
        }

        for _i in 0..self.matmulw_data.in_size {
            match dequeue(&mut self.time, &mut self.matmulw_data.in_stream) {
                Ok(curr_in) => {
                    let mut output_vec = Vec::new();
                    for w_i in weight_regs.iter() {
                        output_vec.push(curr_in.data.dot(w_i));
                    }

                    let output = Array::from_vec(output_vec);
                    let curr_time = self.time.tick();
                    enqueue(
                        &mut self.time,
                        &mut self.matmulw_data.out_stream,
                        ChannelElement::new(
                            curr_time
                                + (self.matmulw_data.systolic_h + self.matmulw_data.systolic_w)
                                    as u64,
                            output,
                        ),
                    )
                    .unwrap();

                    self.time.incr_cycles(1);
                }
                _ => {
                    panic!("Reached unhandled case");
                }
            }
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.matmulw_data.cleanup();
        self.time.cleanup();
    }
}

#[cfg(test)]
mod tests {
    use super::{MatMulW, MatMulWData};
    use crate::{
        context::{
            approx_checker_context::ApproxCheckerContext, generator_context::GeneratorContext,
        },
        simulation::Program,
    };
    use ndarray::{Array, ArrayBase, Dim, OwnedRepr};

    #[test]
    fn matmul_w_unit() {
        const M: usize = 32;
        const N: usize = 16;
        const K: usize = 16;

        let chan_size = 64; // FIFO Depth

        let mut parent = Program::default();

        // Channels
        let (in_sender, in_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>(chan_size);
        let (weight_sender, weight_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>(chan_size);
        let (out_sender, out_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>(chan_size);

        // Generators
        let in_iter = || (0..M).map(|i| Array::from_elem(N, (i as f64) + 1.));
        let weight_iter = || (0..N).map(|_i| Array::from_elem(K, 1.));

        let in_gen = GeneratorContext::new(in_iter, in_sender);
        let weight_gen = GeneratorContext::new(weight_iter, weight_sender);

        // Checker
        let out_iter = || (0..M).map(|i| Array::from_elem(K, ((i as f64) + 1.) * (N as f64)));
        let checker = ApproxCheckerContext::new(out_iter, out_receiver, |a, b| {
            (a - b).to_vec().iter().sum::<f64>() < 0.001
        });

        // Create the MatMul block
        let data = MatMulWData::<f64> {
            in_stream: in_receiver,
            weight_stream: weight_receiver,
            out_stream: out_sender,
            in_size: M,
            systolic_h: N,
            systolic_w: K,
        };

        let matmul_w0 = MatMulW::new(data);

        parent.add_child(in_gen);
        parent.add_child(weight_gen);
        parent.add_child(checker);
        parent.add_child(matmul_w0);
        parent.init();
        parent.run();
    }
}
