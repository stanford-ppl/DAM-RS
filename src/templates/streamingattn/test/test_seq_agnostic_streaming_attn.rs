#[cfg(test)]
mod tests {
    use crate::{
        context::{checker_context::FpBoundedCheckerContext, generator_context::GeneratorContext},
        simulation::Program,
        templates::streamingattn::{
            stream_binary::{BinaryDataElemwise, BinaryOp, BinaryOpType},
            stream_outer_product::{MatmulData, MatmulOuter},
            stream_qkt::{QKTData, QKT},
            stream_reduce::{IncrReduceOp, ReduceData2, ReduceOpType},
        },
    };
    use ndarray::{Array, ArrayBase, Dim, OwnedRepr};

    #[test]
    fn test_seq_agnostic_streaming_attn() {
        // Test Configuration
        const HEAD_DIM: usize = 16;
        const INIT_INTERVAL: u64 = 1;
        const SEQ_LEN: u64 = 5;
        const SEQ_LEN_I32: i32 = 5; // I32 types for generating data
        const HEAD_DIM_I32: i32 = 16; // I32 types for generating data
        const SEQ_LEN_F32: f32 = 5.; // I32 types for generating data
        const HEAD_DIM_F32: f32 = 16.; // I32 types for generating data

        let fifo_chan_size: usize = 4;

        let mut parent = Program::default();

        // Generator
        let (q_sender, q_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f32>, Dim<[usize; 1]>>>(fifo_chan_size);
        let (kt_sender, kt_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f32>, Dim<[usize; 1]>>>(fifo_chan_size);
        // let (v_sender, v_receiver) =
        //     parent.bounded::<ArrayBase<OwnedRepr<f32>, Dim<[usize; 1]>>>(fifo_chan_size);

        let q_iter = || (0..(SEQ_LEN_I32)).map(|i| Array::from_elem(HEAD_DIM, (i as f32) + 1.));
        let kt_iter = || {
            (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|i| {
                if i % SEQ_LEN_I32 == 0 {
                    Array::from_elem(HEAD_DIM, 1.1_f32)
                } else {
                    Array::from_elem(HEAD_DIM, 1_f32)
                }
            })
        };
        // let v_iter = || {
        //     (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|_i| Array::from_elem(HEAD_DIM, SEQ_LEN_F32 - 1.))
        // };

        let q_gen = GeneratorContext::new(q_iter, q_sender); // Q : D length vectors
        let kt_gen = GeneratorContext::new(kt_iter, kt_sender); // KT: D length vectors
                                                                //let v_gen = GeneratorContext::new(v_iter, v_sender); // KT: D length vectors

        // QKT block
        let (qkt_sender1, qkt_receiver1) = parent.bounded::<f32>(fifo_chan_size);
        let (qkt_sender2, qkt_receiver2) = parent.bounded::<f32>(fifo_chan_size);
        let qkt_data = QKTData::<f32> {
            q: q_receiver,
            kt: kt_receiver,
            out_fifo: vec![qkt_sender1, qkt_sender2],
            latency: 1 + (HEAD_DIM_I32.ilog2() as u64),
            init_inverval: INIT_INTERVAL,
            seq_len: SEQ_LEN,
        };
        let stream_qkt = QKT::new(qkt_data);

        // Max
        let (new_max_sender, new_max_receiver) = parent.bounded::<f32>(fifo_chan_size);
        let (old_new_diff_sender, old_new_diff_receiver) = parent.bounded::<f32>(fifo_chan_size);
        let incr_max_data = ReduceData2::<f32> {
            in_stream: qkt_receiver1,
            new_max: new_max_sender,
            old_new_diff: old_new_diff_sender,
            latency: 2,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };
        let stream_incr_max = IncrReduceOp::new(incr_max_data, ReduceOpType::Max);

        // Sub
        let (sub_sender, sub_receiver) = parent.bounded::<f32>(fifo_chan_size);
        let sub_data = BinaryDataElemwise::<f32> {
            in1_stream: qkt_receiver2,
            in2_stream: new_max_receiver,
            out_stream: sub_sender,
            latency: 1,
            init_inverval: INIT_INTERVAL,
            loop_bound: SEQ_LEN * SEQ_LEN,
        };
        let binary_sub = BinaryOp::new(sub_data, BinaryOpType::Sub);

        // Checker
        let out_iter1 = || {
            (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|i| {
                if i % SEQ_LEN_I32 == 0 {
                    0.
                } else {
                    -0.1 * (((i / SEQ_LEN_I32) as f32) + 1.) * HEAD_DIM_F32
                }
            })
        };
        let out_iter2 = || (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|_i| 0.);
        let out_checker1 = FpBoundedCheckerContext::new(out_iter1, sub_receiver, 0.0002_f32);
        let out_checker2 =
            FpBoundedCheckerContext::new(out_iter2, old_new_diff_receiver, 0.0002_f32);

        // Create the Iterators for Checkers

        parent.add_child(q_gen);
        parent.add_child(kt_gen);
        //parent.add_child(v_gen);
        parent.add_child(out_checker1);
        parent.add_child(out_checker2);
        parent.add_child(stream_qkt);
        parent.add_child(stream_incr_max);
        parent.add_child(binary_sub);
        parent.init();
        parent.print_graph();
        parent.run();
    }
}
