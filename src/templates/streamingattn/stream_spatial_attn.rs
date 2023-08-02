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
    templates::streamingattn::stream_binary::BinaryDataOut1,
    types::{Cleanable, DAMType},
};

pub struct QKTExpData<A: Clone> {
    // Performs dot product on two vectors and has one output FIFO
    pub q: Receiver<A>,           // operand 1: Vector
    pub kt: Receiver<A>,          // operand 2: Vector
    pub out_fifo: Vec<Sender<A>>, // list of output scalar FIFOs
    pub latency: u64,             // pipeline depth
    pub init_inverval: u64,       // initiation interval
    pub seq_len: u64,
}

impl<A: DAMType> Cleanable for QKTExpData<A> {
    fn cleanup(&mut self) {
        self.q.cleanup();
        self.kt.cleanup();
        for i in self.out_fifo.iter_mut() {
            i.cleanup();
        }
    }
}

#[time_managed]
#[identifiable]
pub struct QKTExp<A: Clone> {
    qkt_exp_data: QKTExpData<A>,
}

impl<A: DAMType> QKTExp<A>
where
    QKTExp<A>: Context,
    ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>: DAMType,
{
    pub fn new(qkt_exp_data: QKTExpData<A>) -> Self {
        let qkt_exp = QKTExp {
            qkt_exp_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (qkt_exp.qkt_exp_data.q).attach_receiver(&qkt_exp);
        (qkt_exp.qkt_exp_data.kt).attach_receiver(&qkt_exp);
        for i in qkt_exp.qkt_exp_data.out_fifo.iter() {
            i.attach_sender(&qkt_exp);
        }

        qkt_exp
    }
}

impl<A> Context for QKTExp<A>
where
    A: DAMType + num::Float,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        for _i in 0..self.qkt_exp_data.seq_len {
            let q_deq = dequeue(&mut self.time, &mut self.qkt_exp_data.q);
            match q_deq {
                Ok(q) => {
                    for _i in 0..self.qkt_exp_data.seq_len {
                        let kt_deq = dequeue(&mut self.time, &mut self.qkt_exp_data.kt);
                        match kt_deq {
                            Ok(kt) => {
                                let qkt_exp_res = (q.data * kt.data).exp();
                                let curr_time = self.time.tick();

                                for mut j in self.qkt_exp_data.out_fifo.iter_mut() {
                                    enqueue(
                                        &mut self.time,
                                        &mut j,
                                        ChannelElement::new(
                                            curr_time + self.qkt_exp_data.latency,
                                            qkt_exp_res.clone(),
                                        ),
                                    )
                                    .unwrap();
                                }

                                self.time.incr_cycles(self.qkt_exp_data.init_inverval);
                                // initiation interval
                            }
                            _ => {
                                panic!("Reached unhandled case");
                            }
                        }
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
        self.qkt_exp_data.cleanup();
        self.time.cleanup();

        //let curr_time = self.time.tick();
        //println!("QKT exp");
        //dbg!(curr_time);
    }
}

#[time_managed]
#[identifiable]
pub struct MatVecProd<A: Clone> {
    mat_vec_data: BinaryDataOut1<A>,
}

impl<A: DAMType> MatVecProd<A>
where
    MatVecProd<A>: Context,
{
    pub fn new(mat_vec_data: BinaryDataOut1<A>) -> Self {
        let matmul_outer = MatVecProd {
            mat_vec_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (matmul_outer.mat_vec_data.in1_stream).attach_receiver(&matmul_outer);
        (matmul_outer.mat_vec_data.in2_stream).attach_receiver(&matmul_outer);
        (matmul_outer.mat_vec_data.out1_stream).attach_sender(&matmul_outer);

        matmul_outer
    }
}

impl<A> Context for MatVecProd<A>
where
    A: DAMType + num::Num + Copy,
{
    fn init(&mut self) {}
    fn run(&mut self) -> () {
        for _i in 0..self.mat_vec_data.outer_loop_bound {
            let s_deq = dequeue(&mut self.time, &mut self.mat_vec_data.in1_stream);
            let v_deq = dequeue(&mut self.time, &mut self.mat_vec_data.in2_stream);

            match (s_deq, v_deq) {
                (Ok(s_elem), Ok(v_elem)) => {
                    let s_data = s_elem.data;
                    let v_data = v_elem.data;
                    let mut accum_sum = s_data * v_data;

                    self.time.incr_cycles(self.mat_vec_data.init_inverval);

                    for i in 1..self.mat_vec_data.inner_loop_bound {
                        let s_deq = dequeue(&mut self.time, &mut self.mat_vec_data.in1_stream);
                        let v_deq = dequeue(&mut self.time, &mut self.mat_vec_data.in2_stream);

                        match (s_deq, v_deq) {
                            (Ok(s_elem), Ok(v_elem)) => {
                                let s_data = s_elem.data;
                                let v_data = v_elem.data;
                                accum_sum = accum_sum + s_data * v_data;
                            }
                            (_, _) => {
                                panic!("Reached unhandled case");
                            }
                        }
                        if i == self.mat_vec_data.inner_loop_bound - 1 {
                            let curr_time = self.time.tick();
                            enqueue(
                                &mut self.time,
                                &mut self.mat_vec_data.out1_stream,
                                ChannelElement::new(
                                    curr_time + self.mat_vec_data.latency,
                                    accum_sum.clone(),
                                ),
                            )
                            .unwrap();
                        }
                        self.time.incr_cycles(self.mat_vec_data.init_inverval);
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
        self.mat_vec_data.cleanup();
        self.time.cleanup();

        //let curr_time = self.time.tick();
        //println!("Mat_Vec");
        //dbg!(curr_time);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        context::{
            approx_checker_context::ApproxCheckerContext, generator_context::GeneratorContext,
        },
        simulation::Program,
        templates::streamingattn::stream_binary::BinaryDataOut1,
    };

    use super::{MatVecProd, QKTExp, QKTExpData};

    #[test]
    fn stream_qkt_exp_test() {
        const LATENCY: u64 = 1;
        const INIT_INTERVAL: u64 = 1;
        const SEQ_LEN: u64 = 5;

        // We will use seq length of 5 for now. So, conservatively keep the FIFO (Channel) size as 5.
        let chan_size = 2; // FIFO Depth

        let mut parent = Program::default();

        // Create Generators
        let (in1_sender, in1_receiver) = parent.bounded::<f64>(chan_size);
        let (in2_sender, in2_receiver) = parent.bounded::<f64>(chan_size);

        let in1_iter = || (0..(SEQ_LEN)).map(|_i| 1_f64);
        let in2_iter = || (0..(SEQ_LEN * SEQ_LEN)).map(|_i| 1_f64);
        let gen1 = GeneratorContext::new(in1_iter, in1_sender); // Q : [1,D] shaped vectors
        let gen2 = GeneratorContext::new(in2_iter, in2_sender); // KT: [D,1] shaped vectors

        // Create the QKT block
        let (out_sender, out_receiver) = parent.bounded::<f64>(chan_size);
        let data = QKTExpData::<f64> {
            q: in1_receiver,
            kt: in2_receiver,
            out_fifo: vec![out_sender],
            latency: LATENCY,
            init_inverval: INIT_INTERVAL,
            seq_len: SEQ_LEN,
        };
        let stream_qkt_exp = QKTExp::new(data);

        // Create the Iterators for Checkers
        let out_iter = || (0..(SEQ_LEN * SEQ_LEN)).map(|_i| (1_f64).exp());
        let out_checker =
            ApproxCheckerContext::new(out_iter, out_receiver, |a, b| (a - b).abs() < 0.0001);

        parent.add_child(gen1);
        parent.add_child(gen2);
        parent.add_child(stream_qkt_exp);
        parent.add_child(out_checker);
        parent.init();
        parent.run();
    }

    #[test]
    fn stream_mat_vec_prod_test() {
        const LATENCY: u64 = 1;
        const INIT_INTERVAL: u64 = 1;
        const SEQ_LEN: u64 = 16;

        const SEQ_LEN_F64: f64 = 16.;
        let chan_size = 2; // FIFO Depth

        let mut parent = Program::default();

        // Create Generators
        let (in1_sender, in1_receiver) = parent.bounded::<f64>(chan_size);
        let (in2_sender, in2_receiver) = parent.bounded::<f64>(chan_size);

        let in1_iter = || (0..(SEQ_LEN * SEQ_LEN)).map(|_i| 1_f64);
        let in2_iter = || (0..(SEQ_LEN * SEQ_LEN)).map(|_i| 1_f64);
        let gen1 = GeneratorContext::new(in1_iter, in1_sender); // Q : [1,D] shaped vectors
        let gen2 = GeneratorContext::new(in2_iter, in2_sender); // KT: [D,1] shaped vectors

        // Create the MatVecProduct
        let (out_sender, out_receiver) = parent.bounded::<f64>(chan_size);
        let data = BinaryDataOut1::<f64> {
            in1_stream: in1_receiver,
            in2_stream: in2_receiver,
            out1_stream: out_sender,
            latency: LATENCY,
            init_inverval: INIT_INTERVAL,
            inner_loop_bound: SEQ_LEN,
            outer_loop_bound: SEQ_LEN,
        };
        let stream_mat_vec_prod = MatVecProd::new(data);

        // Create the Iterators for Checkers
        let out_iter = || (0..(SEQ_LEN)).map(|_i| (1_f64) * SEQ_LEN_F64);
        let out_checker =
            ApproxCheckerContext::new(out_iter, out_receiver, |a, b| (a - b).abs() < 0.0001);

        parent.add_child(gen1);
        parent.add_child(gen2);
        parent.add_child(stream_mat_vec_prod);
        parent.add_child(out_checker);
        parent.init();
        parent.run();
    }
}
