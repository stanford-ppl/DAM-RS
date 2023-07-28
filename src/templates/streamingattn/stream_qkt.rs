use dam_core::identifier::Identifier;
use dam_core::TimeManager;
use dam_macros::{cleanup, identifiable, time_managed};

use ndarray::{ArrayBase, Dim, OwnedRepr};

use crate::{
    channel::{
        utils::{dequeue, enqueue, peek_next},
        ChannelElement, Receiver, Sender,
    },
    context::Context,
    types::{Cleanable, DAMType},
};

pub struct QKTData<A: Clone> {
    pub q: Receiver<ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>,
    pub kt: Receiver<ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>,
    pub out: Sender<ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>,
    pub latency: u64,       // pipeline depth
    pub init_inverval: u64, // initiation interval
    pub seq_len: u64,
}

impl<A: DAMType> Cleanable for QKTData<A>
where
    ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>: DAMType,
{
    fn cleanup(&mut self) {
        self.q.cleanup();
        self.kt.cleanup();
        self.out.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct QKT<A: Clone> {
    qkt_data: QKTData<A>,
    //latency: dam_core::time::Time,
}

impl<A: DAMType> QKT<A>
where
    QKT<A>: Context,
    ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>: DAMType,
{
    pub fn new(qkt_data: QKTData<A>) -> Self {
        let qkt = QKT {
            qkt_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (qkt.qkt_data.q).attach_receiver(&qkt);
        (qkt.qkt_data.kt).attach_receiver(&qkt);
        (qkt.qkt_data.out).attach_sender(&qkt);

        qkt
    }
}

impl<A> Context for QKT<A>
where
    ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>: DAMType
        //+ std::ops::Mul<Output = ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>
        + ndarray::linalg::Dot<
            ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>,
            Output = ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>,
        >,
    A: DAMType + num::Num + std::iter::Sum,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        for _i in 0..self.qkt_data.seq_len {
            let q_deq = dequeue(&mut self.time, &mut self.qkt_data.q);
            match q_deq {
                Ok(q) => {
                    for _i in 0..self.qkt_data.seq_len {
                        let kt_deq = peek_next(&mut self.time, &mut self.qkt_data.kt);
                        match kt_deq {
                            Ok(kt) => {
                                let reduce_sum = q.data.dot(&(kt.data));
                                let curr_time = self.time.tick();
                                enqueue(
                                    &mut self.time,
                                    &mut self.qkt_data.out,
                                    ChannelElement::new(
                                        curr_time + self.qkt_data.latency,
                                        reduce_sum,
                                    ),
                                )
                                .unwrap();
                                dequeue(&mut self.time, &mut self.qkt_data.kt).unwrap();
                                self.time.incr_cycles(self.qkt_data.init_inverval);
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
        self.qkt_data.cleanup();
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

    use super::{QKTData, QKT};

    #[test]
    fn stream_qkt_test() {
        const HEAD_DIM: usize = 16;
        const LATENCY: u64 = 1 + (HEAD_DIM_I32.ilog2() as u64);
        const INIT_INTERVAL: u64 = 1;
        const SEQ_LEN: u64 = 5;

        // I32 types for generating data
        const SEQ_LEN_I32: i32 = 5;
        const HEAD_DIM_I32: i32 = 16;

        // We will use seq length of 5 for now. So, conservatively keep the FIFO (Channel) size as 5.
        let chan_size = 30; // FIFO Depth

        let mut parent = Program::default();

        // Create Channels
        let (in1_sender, in1_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>>(chan_size);
        let (in2_sender, in2_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>>(chan_size);
        let (out_sender, out_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>>(chan_size);

        // Create the QKT block
        let data = QKTData::<i32> {
            q: in1_receiver,
            kt: in2_receiver,
            out: out_sender,
            latency: LATENCY,
            init_inverval: INIT_INTERVAL,
            seq_len: SEQ_LEN,
        };

        let stream_qkt = QKT::new(data);

        // Create the Iterators for Generators
        let in1_iter = || {
            (0..(SEQ_LEN_I32))
                .map(|i| array![[(i + 1); HEAD_DIM]])
                .collect::<Vec<_>>()
                .into_iter()
        };
        let in2_iter = || {
            (0..(SEQ_LEN_I32 * SEQ_LEN_I32))
                .map(|_i| array![[1; HEAD_DIM]].into_shape([HEAD_DIM, 1]).unwrap())
                .collect::<Vec<_>>()
                .into_iter()
        };

        // Create the Iterators for Checkers
        let out_iter = || {
            (0..(SEQ_LEN_I32 * SEQ_LEN_I32))
                .map(|i| array![[(i / SEQ_LEN_I32 + 1) * HEAD_DIM_I32]])
                .collect::<Vec<_>>()
                .into_iter()
        };

        let gen1 = GeneratorContext::new(in1_iter, in1_sender); // Q : [1,D] shaped vectors
        let gen2 = GeneratorContext::new(in2_iter, in2_sender); // KT: [D,1] shaped vectors

        let qkt_checker = CheckerContext::new(out_iter, out_receiver);

        parent.add_child(gen1);
        parent.add_child(gen2);
        parent.add_child(qkt_checker);
        parent.add_child(stream_qkt);
        parent.init();
        parent.run();
    }
}
