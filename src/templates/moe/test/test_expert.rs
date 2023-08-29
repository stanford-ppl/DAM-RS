#[cfg(test)]
mod tests {
    use crate::{
        context::{
            approx_checker_context::ApproxCheckerContext, generator_context::GeneratorContext,
        },
        simulation::Program,
        templates::{
            llops::matmul_w::{MatMulW, MatMulWData},
            pcu::{PCUConfig, PipelineStage, PCU},
            vec_ops::*,
        },
    };
    use ndarray::{Array, ArrayBase, Dim, OwnedRepr};

    #[test]
    fn test_relu() {
        const M: usize = 32;
        const N: usize = 16;
        const K: usize = 16;

        let chan_size = 64; // FIFO Depth

        let mut parent = Program::default();

        // Generator Channels
        let (in_sender, in_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>(chan_size);
        let (zero_sender, zero_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>(chan_size);

        // Input Generators
        let in_iter = || {
            (0..M).map(|i| {
                if i % 2 == 0 {
                    Array::from_elem(N, (i as f64) + 1.)
                } else {
                    Array::from_elem(N, (-1.) * ((i as f64) + 1.))
                }
            })
        }; // Input: M x N
        let zero_iter = || (0..M).map(|_i| Array::from_elem(N, 0.)); // Input: M x N

        let in_gen = GeneratorContext::new(in_iter, in_sender);
        let zero_gen = GeneratorContext::new(zero_iter, zero_sender);

        // Checker Channels
        let (out_sender, out_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>(chan_size);

        // Checker
        let out_iter = || {
            (0..M).map(|i| {
                if i % 2 == 0 {
                    Array::from_elem(N, (i as f64) + 1.)
                } else {
                    Array::from_elem(N, 0.)
                }
            })
        }; // M x P
        let checker = ApproxCheckerContext::new(out_iter, out_receiver, |a, b| {
            (a - b).to_vec().iter().sum::<f64>() < 0.001
        });

        // Relu
        //  unless we make a map node with a constant input instead of a stream, the 0 has to be repeated
        //  I will use generator for now.
        let ingress_op = PCU::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>::READ_ALL_INPUTS;
        let egress_op = PCU::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>::WRITE_ALL_RESULTS;

        let mut pcu = PCU::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>::new(
            PCUConfig {
                pipeline_depth: 1,
                num_registers: 2,
            },
            ingress_op,
            egress_op,
        );

        pcu.push_stage(PipelineStage {
            op: ALUVecMaxOp::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>(),
            forward: vec![],
            prev_register_ids: vec![0, 1],
            next_register_ids: vec![],
            output_register_ids: vec![0],
        });

        pcu.add_input_channel(in_receiver);
        pcu.add_input_channel(zero_receiver);
        pcu.add_output_channel(out_sender);

        parent.add_child(in_gen);
        parent.add_child(zero_gen);
        parent.add_child(checker);
        parent.add_child(pcu);
        parent.init();
        parent.run();
    }

    #[test]
    fn test_expert() {
        const M: usize = 32;
        const N: usize = 16;
        const K: usize = 16;
        const P: usize = 16;

        let chan_size = 64; // FIFO Depth

        let mut parent = Program::default();

        // Generator Channels
        let (in_sender, in_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>(chan_size);
        let (weight0_sender, weight0_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>(chan_size);
        let (weight1_sender, weight1_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>(chan_size);
        let (zero_sender, zero_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>(chan_size);

        // Generators
        let in_iter = || (0..M).map(|i| Array::from_elem(N, (i as f64) + 1.)); // Input: M x N
        let weight0_iter = || (0..N).map(|_i| Array::from_elem(K, 1.)); // w0: N x K
        let weight1_iter = || (0..K).map(|_i| Array::from_elem(P, 1.)); // w1: K x P
        let zero_iter = || (0..M).map(|_i| Array::from_elem(K, 0.)); // Input: M x K

        let in_gen = GeneratorContext::new(in_iter, in_sender);
        let weight0_gen = GeneratorContext::new(weight0_iter, weight0_sender);
        let weight1_gen = GeneratorContext::new(weight1_iter, weight1_sender);
        let zero_gen = GeneratorContext::new(zero_iter, zero_sender);

        // Checker Channels
        let (out_sender, out_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>(chan_size);

        // Checker
        let out_iter =
            || (0..M).map(|i| Array::from_elem(P, ((i as f64) + 1.) * (N as f64 * K as f64))); // M x P
        let checker = ApproxCheckerContext::new(out_iter, out_receiver, |a, b| {
            (a - b).to_vec().iter().sum::<f64>() < 0.001
        });

        // Matmul (w0)
        let (matmul_w0_sender, matmul_w0_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>(chan_size);
        let data_w0 = MatMulWData::<f64> {
            in_stream: in_receiver,
            weight_stream: weight0_receiver,
            out_stream: matmul_w0_sender,
            in_size: M,
            systolic_h: N,
            systolic_w: K,
        };

        let matmul_w0 = MatMulW::new(data_w0);

        // Relu
        //  unless we make a map node with a constant input instead of a stream, the 0 has to be repeated
        //  I will use generator for now.
        let (relu_sender, relu_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>(chan_size);
        let ingress_op = PCU::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>::READ_ALL_INPUTS;
        let egress_op = PCU::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>::WRITE_ALL_RESULTS;

        let mut pcu = PCU::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>::new(
            PCUConfig {
                pipeline_depth: 1,
                num_registers: 2,
            },
            ingress_op,
            egress_op,
        );

        pcu.push_stage(PipelineStage {
            op: ALUVecMaxOp::<ArrayBase<OwnedRepr<f64>, Dim<[usize; 1]>>>(),
            forward: vec![],
            prev_register_ids: vec![0, 1],
            next_register_ids: vec![],
            output_register_ids: vec![0],
        });

        pcu.add_input_channel(matmul_w0_receiver);
        pcu.add_input_channel(zero_receiver);
        pcu.add_output_channel(relu_sender);

        // Matmul (w1)
        let data_w1 = MatMulWData::<f64> {
            in_stream: relu_receiver,
            weight_stream: weight1_receiver,
            out_stream: out_sender,
            in_size: M,
            systolic_h: K,
            systolic_w: P,
        };

        let matmul_w1 = MatMulW::new(data_w1);

        parent.add_child(in_gen);
        parent.add_child(weight0_gen);
        parent.add_child(weight1_gen);
        parent.add_child(zero_gen);
        parent.add_child(checker);
        parent.add_child(matmul_w0);
        parent.add_child(matmul_w1);
        parent.add_child(pcu);
        parent.set_inference(true); // turn on flavor inference
        parent.print_graph_with_names();
        parent.init();
        parent.run();
    }
}
