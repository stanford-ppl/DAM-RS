use core::fmt::Debug;
use dam_core::identifier::Identifier;
use dam_core::TimeManager;
use dam_macros::{cleanup, identifiable, time_managed};

use ndarray::prelude::*;
use ndarray::{Array, ArrayBase, Axis, Dim, Dimension, OwnedRepr};

use crate::{
    channel::{
        utils::{dequeue, enqueue, peek_next},
        ChannelElement, Receiver, Sender,
    },
    context::Context,
    types::{Cleanable, DAMType},
};

pub struct QKTData<A: Clone> {
    pub in1: Receiver<ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>,
    pub in2: Receiver<ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>,
    pub out: Sender<ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>,
}

impl<A: Clone> Cleanable for QKTData<A>
where
    ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>: DAMType,
{
    fn cleanup(&mut self) {
        self.in1.cleanup();
        self.in2.cleanup();
        self.out.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct QKT<A: Clone> {
    qkt_data: QKTData<A>,
    //latency: dam_core::time::Time,
}

impl<A: Clone> QKT<A>
where
    QKT<A>: Context,
    ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>: DAMType,
{
    pub fn new(qkt_data: QKTData<A>) -> Self {
        let qkt = QKT {
            qkt_data,
            // latency,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (qkt.qkt_data.in1).attach_receiver(&qkt);
        (qkt.qkt_data.in2).attach_receiver(&qkt);
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
    A: Clone + num::Num + std::iter::Sum,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        let mut count = 0u32;
        loop {
            let in1_deq = peek_next(&mut self.time, &mut self.qkt_data.in1);
            let in2_deq = peek_next(&mut self.time, &mut self.qkt_data.in2);

            match (in1_deq, in2_deq) {
                (Ok(in1), Ok(in2)) => {
                    // let mul_in12: ArrayBase<OwnedRepr<A>, D> = in1.data.clone() * in2.data;
                    // // we clone as the multiplication consumes the first operand
                    // let reduce_sum_small =
                    //     mul_in12.map_axis(Axis(0), |view| view.to_owned().into_iter().sum::<A>());
                    // let reduced_len = reduce_sum_small.len();
                    // let reduce_sum = reduce_sum_small.into_shape((1, reduced_len)).unwrap();
                    let reduce_sum = in1.data.dot(&(in2.data));
                    let curr_time = self.time.tick();
                    enqueue(
                        &mut self.time,
                        &mut self.qkt_data.out,
                        ChannelElement::new(curr_time + 5, reduce_sum),
                        // assuming a pipeline depth = 5 (this could be later parameterized through latency)
                    )
                    .unwrap();
                    dequeue(&mut self.time, &mut self.qkt_data.in1).unwrap();
                    dequeue(&mut self.time, &mut self.qkt_data.in2).unwrap();
                    count += 1;
                    if count == 5 {
                        // can also be parallelized. For now this is the value of N (assuming we only process a single sequence)
                        println!("OK, that's enough");

                        // Exit this loop
                        break;
                    }
                }
                (_, _) => {
                    panic!("Reached unhandled case");
                }
            }
            self.time.incr_cycles(1); // initiation interval (could also be parameterized later)
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
    use ndarray::{array, ArrayBase, Axis, Dim, OwnedRepr};

    use super::{QKTData, QKT};

    #[test]
    fn test_ndarray_clone() {
        // array![1, 1, 1] :shape=[3]
        // array![[1, 1, 1]] :shape=[1,3]
        let array0 = array![1, 1, 1];
        println!("{:?}", array0);

        let array1: ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>> = array![[1, 2, 1], [3, 4, 5]];
        let array12 = array1.sum_axis(Axis(0));
        // let array2 = array![[1, 2, 1], [3, 4, 5]];
        // let array3 = array1.clone() * array2;
        // //println!("{:?}", array1);
        // println!("{:?}", array3);
        // let reduction_arr3 = array3.map_axis(Axis(0), |view| view.iter().sum::<i32>());
        // println!("{:?}", reduction_arr3);

        let darray1_a = array![1, 1, 1];
        let darray1_b = array![1, 1, 1];

        let darray2_a = array![[1, 1, 1]];
        let darray2_b = array![[1, 1, 1]];
    }

    #[test]
    fn stream_qkt_test() {
        // We will use seq length of 5 for now. So, conservatively keep the FIFO (Channel) size as 5.
        let chan_size = 5;

        let mut parent = Program::default();

        // Create Channels
        let (in1_sender, in1_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>>(chan_size);
        let (in2_sender, in2_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>>(chan_size);
        let (out_sender, out_receiver) =
            parent.bounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>>(chan_size);

        let data = QKTData::<i32> {
            in1: in1_receiver,
            in2: in2_receiver,
            out: out_sender,
        };

        let stream_qkt = QKT::new(data);
    }
}
