#[cfg(test)]
mod tests {
    use crate::{
        context::{
            approx_checker_context::ApproxCheckerContext, generator_context::GeneratorContext,
        },
        simulation::Program,
        templates::streamingattn::{
            stream_binary::{BinaryDataOut1, BinaryOpOut1, BinaryOpType},
            stream_reduce::{ReduceData, ReduceOp, ReduceOpType},
            stream_spatial_attn::{MatVecProd, QKTExp, QKTExpData},
        },
    };

    #[test]
    fn stream_spatial_streamed_attn() {
        const LATENCY: u64 = 1;
        const INIT_INTERVAL: u64 = 1;

        const SEQ_LEN: u64 = 512;
        const SEQ_LEN_F64: f64 = 512.;
        let chan_size_long = 514;

        let chan_size = 2; // FIFO Depth

        let mut parent = Program::default();

        // Generators
        let (q_sender, q_receiver) = parent.bounded::<f64>(chan_size);
        let (kt_sender, kt_receiver) = parent.bounded::<f64>(chan_size);
        let (v_sender, v_receiver) = parent.bounded::<f64>(chan_size);
        let q_iter = || (0..(SEQ_LEN)).map(|i| (i as f64) * 0.01_f64);
        let kt_iter =
            || (0..(SEQ_LEN * SEQ_LEN)).map(|i| if i % SEQ_LEN == 0 { 0.11_f64 } else { 0.1_f64 });
        let v_iter = || (0..(SEQ_LEN * SEQ_LEN)).map(|_i| 1_f64);
        let q_gen = GeneratorContext::new(q_iter, q_sender); // Q : [1,D] shaped vectors
        let kt_gen = GeneratorContext::new(kt_iter, kt_sender); // KT: [D,1] shaped vectors
        let v_gen = GeneratorContext::new(v_iter, v_sender); // KT: [D,1] shaped vectors

        // QKT & Exp block
        let (qkt_exp_short_sender, qkt_exp_short_receiver) = parent.bounded::<f64>(chan_size);
        let (qkt_exp_long_sender, qkt_exp_long_receiver) = parent.bounded::<f64>(chan_size_long);
        let qkt_exp_data = QKTExpData::<f64> {
            q: q_receiver,
            kt: kt_receiver,
            out_fifo: vec![qkt_exp_short_sender, qkt_exp_long_sender],
            latency: LATENCY,
            init_inverval: INIT_INTERVAL,
            seq_len: SEQ_LEN,
        };
        let stream_qkt_exp = QKTExp::new(qkt_exp_data);

        // Reduce
        let (rowsum_sender, rowsum_receiver) = parent.bounded::<f64>(chan_size);
        let reduce_data = ReduceData::<f64> {
            in_stream: qkt_exp_short_receiver,
            out_stream: rowsum_sender,
            latency: LATENCY,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };
        let stream_reduce = ReduceOp::new(reduce_data, ReduceOpType::Sum);

        // Div
        let (div_sender, div_receiver) = parent.bounded::<f64>(chan_size);
        let div_data = BinaryDataOut1::<f64> {
            in1_stream: qkt_exp_long_receiver,
            in2_stream: rowsum_receiver,
            out1_stream: div_sender,
            latency: LATENCY,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };
        let stream_div = BinaryOpOut1::new(div_data, BinaryOpType::Div);

        // Multiply with V
        let (out_sender, out_receiver) = parent.bounded::<f64>(chan_size);
        let mat_vec_data = BinaryDataOut1::<f64> {
            in1_stream: div_receiver,
            in2_stream: v_receiver,
            out1_stream: out_sender,
            latency: LATENCY,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };
        let stream_mat_vec_prod = MatVecProd::new(mat_vec_data);

        // Checkers
        let out_iter = || (0..(SEQ_LEN)).map(|_i| (1_f64));
        let out_checker =
            ApproxCheckerContext::new(out_iter, out_receiver, |a, b| (a - b).abs() < 0.0001);

        parent.add_child(q_gen);
        parent.add_child(kt_gen);
        parent.add_child(v_gen);
        parent.add_child(stream_qkt_exp);
        parent.add_child(stream_reduce);
        parent.add_child(stream_div);
        parent.add_child(stream_mat_vec_prod);
        parent.add_child(out_checker);
        // parent.set_inference(true); // turn on flavor inference
        parent.init();
        parent.run();
        let finish_time = parent.elapsed_cycles();
        dbg!(finish_time);
    }
}
