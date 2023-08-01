#[cfg(test)]
mod tests {
    use crate::{
        context::{checker_context::CheckerContext, generator_context::GeneratorContext},
        simulation::Program,
        templates::streamingattn::{
            stream_binary::{
                BinaryDataOut1, BinaryDataOut2, BinaryOpOut1, BinaryOpOut2, BinaryOpType,
            },
            stream_outer_product::{MatmulData, MatmulOuter},
            stream_qkt::{QKTDataOut2, QKTOut2},
            stream_reduce::{ReduceData, ReduceOp, ReduceOpType},
        },
    };
    use ndarray::{Array, ArrayBase, Dim, OwnedRepr};

    #[test]
    fn test_atten() {
        // Test Configuration
        const HEAD_DIM: usize = 16;
        const INIT_INTERVAL: u64 = 1;
        const SEQ_LEN: u64 = 5;
        const SEQ_LEN_I32: i32 = 5; // I32 types for generating data
        const HEAD_DIM_I32: i32 = 16; // I32 types for generating data

        let short_fifo_chan_size = 2;
        let long_fifo_chan_size = 30;

        let mut parent = Program::default();

        // Generator
        let (q_sender, q_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 1]>>>(short_fifo_chan_size);
        let (kt_sender, kt_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 1]>>>(short_fifo_chan_size);
        let (v_sender, v_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 1]>>>(short_fifo_chan_size);

        let q_iter = || (0..(SEQ_LEN_I32)).map(|i| Array::from_elem(HEAD_DIM, i + 1));
        let kt_iter = || {
            (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|i| {
                if i % SEQ_LEN_I32 == 0 {
                    Array::from_elem(HEAD_DIM, 2)
                } else {
                    Array::from_elem(HEAD_DIM, 1)
                }
            })
        };
        let v_iter = || {
            (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|_i| Array::from_elem(HEAD_DIM, SEQ_LEN_I32 - 1))
        };

        let q_gen = GeneratorContext::new(q_iter, q_sender); // Q : D length vectors
        let kt_gen = GeneratorContext::new(kt_iter, kt_sender); // KT: D length vectors
        let v_gen = GeneratorContext::new(v_iter, v_sender); // KT: D length vectors

        // QKT block
        let (qkt_short_sender, qkt_short_receiver) = parent.bounded::<i32>(short_fifo_chan_size);
        let (qkt_long_sender, qkt_long_receiver) = parent.bounded::<i32>(long_fifo_chan_size);
        let qkt_data = QKTDataOut2::<i32> {
            q: q_receiver,
            kt: kt_receiver,
            out_fifo_short: qkt_short_sender,
            out_fifo_long: qkt_long_sender,
            latency: 1 + (HEAD_DIM_I32.ilog2() as u64),
            init_inverval: INIT_INTERVAL,
            seq_len: SEQ_LEN,
        };
        let stream_qkt = QKTOut2::new(qkt_data);

        // RowMax
        let (rowmax_sender, rowmax_receiver) = parent.bounded::<i32>(short_fifo_chan_size);
        let rowmax_data = ReduceData::<i32> {
            in_stream: qkt_short_receiver,
            out_stream: rowmax_sender,
            latency: 1,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };
        let stream_reduce_max = ReduceOp::new(rowmax_data, ReduceOpType::Max);

        // Sub
        let (sub_short_sender, sub_short_receiver) = parent.bounded::<i32>(short_fifo_chan_size);
        let (sub_long_sender, sub_long_receiver) = parent.bounded::<i32>(long_fifo_chan_size);
        let rowmax_data = BinaryDataOut2::<i32> {
            in1_stream: qkt_long_receiver, // operand 1: N*N result of Q*KT
            in2_stream: rowmax_receiver,   // operand 2: Rowmax of operand 1
            out1_stream: sub_short_sender,
            out2_stream: sub_long_sender,
            latency: 1,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };
        let stream_sub_rowmax = BinaryOpOut2::new(rowmax_data, BinaryOpType::Sub);

        // RowSum
        let (rowsum_sender, rowsum_receiver) = parent.bounded::<i32>(short_fifo_chan_size);
        let rowsum_data = ReduceData::<i32> {
            in_stream: sub_short_receiver,
            out_stream: rowsum_sender,
            latency: 1,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };
        let stream_reduce_sum = ReduceOp::new(rowsum_data, ReduceOpType::Sum);

        // Div
        let (div_sender, div_receiver) = parent.bounded::<i32>(short_fifo_chan_size);
        let div_data = BinaryDataOut1::<i32> {
            in1_stream: sub_long_receiver, // operand 1: N*N result of Q*KT
            in2_stream: rowsum_receiver,   // operand 2: Rowmax of operand 1
            out1_stream: div_sender,
            latency: 1,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };
        let stream_div_rowsum = BinaryOpOut1::new(div_data, BinaryOpType::Div);

        // mul V
        let (out_sender, out_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 1]>>>(short_fifo_chan_size);
        let outer_prod_data = MatmulData::<i32> {
            in_scalar_stream: div_receiver,
            in_vec_stream: v_receiver,
            out_stream: out_sender,
            latency: 1,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };
        let stream_outer_product = MatmulOuter::new(outer_prod_data);

        // Checker
        let out_long_iter = || (0..(SEQ_LEN_I32)).map(|_i| Array::from_elem(HEAD_DIM, 0));
        let qkt_short_checker = CheckerContext::new(out_long_iter, out_receiver);

        // Create the Iterators for Checkers

        parent.add_child(q_gen);
        parent.add_child(kt_gen);
        parent.add_child(v_gen);
        parent.add_child(qkt_short_checker);
        parent.add_child(stream_qkt);
        parent.add_child(stream_reduce_max);
        parent.add_child(stream_sub_rowmax);
        parent.add_child(stream_reduce_sum);
        parent.add_child(stream_div_rowsum);
        parent.add_child(stream_outer_product);
        parent.init();
        parent.run();
    }

    #[test]
    fn test_atten_f32() {
        // Test Configuration
        const HEAD_DIM: usize = 16;
        const INIT_INTERVAL: u64 = 1;
        const SEQ_LEN: u64 = 512;
        const SEQ_LEN_I32: i32 = 512; // I32 types for generating data
        const HEAD_DIM_I32: i32 = 16; // I32 types for generating data
        const SEQ_LEN_F32: f32 = 512.; // I32 types for generating data

        let short_fifo_chan_size = 2;
        let long_fifo_chan_size = 513;

        let mut parent = Program::default();

        // Generator
        let (q_sender, q_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f32>, Dim<[usize; 1]>>>(short_fifo_chan_size);
        let (kt_sender, kt_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f32>, Dim<[usize; 1]>>>(short_fifo_chan_size);
        let (v_sender, v_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f32>, Dim<[usize; 1]>>>(short_fifo_chan_size);

        let q_iter = || (0..(SEQ_LEN_I32)).map(|i| Array::from_elem(HEAD_DIM, (i as f32) + 1.));
        let kt_iter = || {
            (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|i| {
                if i % SEQ_LEN_I32 == 0 {
                    Array::from_elem(HEAD_DIM, 2.)
                } else {
                    Array::from_elem(HEAD_DIM, 1.)
                }
            })
        };
        let v_iter = || {
            (0..(SEQ_LEN_I32 * SEQ_LEN_I32)).map(|_i| Array::from_elem(HEAD_DIM, SEQ_LEN_F32 - 1.))
        };

        let q_gen = GeneratorContext::new(q_iter, q_sender); // Q : D length vectors
        let kt_gen = GeneratorContext::new(kt_iter, kt_sender); // KT: D length vectors
        let v_gen = GeneratorContext::new(v_iter, v_sender); // KT: D length vectors

        // QKT block
        let (qkt_short_sender, qkt_short_receiver) = parent.bounded::<f32>(short_fifo_chan_size);
        let (qkt_long_sender, qkt_long_receiver) = parent.bounded::<f32>(long_fifo_chan_size);
        let qkt_data = QKTDataOut2::<f32> {
            q: q_receiver,
            kt: kt_receiver,
            out_fifo_short: qkt_short_sender,
            out_fifo_long: qkt_long_sender,
            latency: 1 + (HEAD_DIM_I32.ilog2() as u64),
            init_inverval: INIT_INTERVAL,
            seq_len: SEQ_LEN,
        };
        let stream_qkt = QKTOut2::new(qkt_data);

        // RowMax
        let (rowmax_sender, rowmax_receiver) = parent.bounded::<f32>(short_fifo_chan_size);
        let rowmax_data = ReduceData::<f32> {
            in_stream: qkt_short_receiver,
            out_stream: rowmax_sender,
            latency: 1,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };
        let stream_reduce_max = ReduceOp::new(rowmax_data, ReduceOpType::Max);

        // Sub
        let (sub_short_sender, sub_short_receiver) = parent.bounded::<f32>(short_fifo_chan_size);
        let (sub_long_sender, sub_long_receiver) = parent.bounded::<f32>(long_fifo_chan_size);
        let rowmax_data = BinaryDataOut2::<f32> {
            in1_stream: qkt_long_receiver, // operand 1: N*N result of Q*KT
            in2_stream: rowmax_receiver,   // operand 2: Rowmax of operand 1
            out1_stream: sub_short_sender,
            out2_stream: sub_long_sender,
            latency: 1,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };
        let stream_sub_rowmax = BinaryOpOut2::new(rowmax_data, BinaryOpType::Sub);

        // RowSum
        let (rowsum_sender, rowsum_receiver) = parent.bounded::<f32>(short_fifo_chan_size);
        let rowsum_data = ReduceData::<f32> {
            in_stream: sub_short_receiver,
            out_stream: rowsum_sender,
            latency: 1,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };
        let stream_reduce_sum = ReduceOp::new(rowsum_data, ReduceOpType::Sum);

        // Div
        let (div_sender, div_receiver) = parent.bounded::<f32>(short_fifo_chan_size);
        let div_data = BinaryDataOut1::<f32> {
            in1_stream: sub_long_receiver, // operand 1: N*N result of Q*KT
            in2_stream: rowsum_receiver,   // operand 2: Rowmax of operand 1
            out1_stream: div_sender,
            latency: 1,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };
        let stream_div_rowsum = BinaryOpOut1::new(div_data, BinaryOpType::Div);

        // mul V
        let (out_sender, out_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<f32>, Dim<[usize; 1]>>>(short_fifo_chan_size);
        let outer_prod_data = MatmulData::<f32> {
            in_scalar_stream: div_receiver,
            in_vec_stream: v_receiver,
            out_stream: out_sender,
            latency: 1,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };
        let stream_outer_product = MatmulOuter::new(outer_prod_data);

        // Checker
        let out_long_iter =
            || (0..(SEQ_LEN_I32)).map(|_i| Array::from_elem(HEAD_DIM, SEQ_LEN_F32 - 1.));
        let qkt_short_checker = CheckerContext::new(out_long_iter, out_receiver);

        // Create the Iterators for Checkers

        parent.add_child(q_gen);
        parent.add_child(kt_gen);
        parent.add_child(v_gen);
        parent.add_child(qkt_short_checker);
        parent.add_child(stream_qkt);
        parent.add_child(stream_reduce_max);
        parent.add_child(stream_sub_rowmax);
        parent.add_child(stream_reduce_sum);
        parent.add_child(stream_div_rowsum);
        parent.add_child(stream_outer_product);
        parent.init();
        parent.run();
    }

    #[test]
    fn test_min_func() {
        let a = 2;
        let b = 4;

        let c: f64 = 2.;
        let d: f64 = 4.;

        assert_eq!(a.min(b), a);
        assert_eq!(c.min(d), c);
    }
}
