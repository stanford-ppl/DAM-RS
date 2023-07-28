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
    pub in1: Receiver<ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>,
    pub in2: Receiver<ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>,
    pub out: Sender<ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>,
}

impl<A: DAMType> Cleanable for QKTData<A>
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

impl<A: DAMType> QKT<A>
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
    A: DAMType + num::Num + std::iter::Sum,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        let mut count = 0u32;
        loop {
            let in1_deq = peek_next(&mut self.time, &mut self.qkt_data.in1);
            let in2_deq = peek_next(&mut self.time, &mut self.qkt_data.in2);

            match (in1_deq, in2_deq) {
                (Ok(in1), Ok(in2)) => {
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
    fn test_ndarrays() {
        // Checking array shapes
        // - array![1, 1, 1] :shape=[3]
        // - array![[1, 1, 1]] :shape=[1,3]
        let array0 = array![1, 1, 1];
        println!("{:?}", array0);

        let array_s = array![[1]];
        println!("{:?}", array_s);

        // Testing sum_axis
        let array1: ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>> = array![[1, 2, 1], [3, 4, 5]];
        let array1_sum = array1.sum_axis(Axis(0));
        println!("{:?}", array1_sum);

        // Testing map_axis
        let array3: ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>> = array![[1, 2, 1], [3, 4, 5]];
        let reduction_arr3 = array3.map_axis(Axis(0), |view| view.iter().sum::<i32>());
        println!("{:?}", reduction_arr3);
    }

    #[test]
    fn test_iterator() {
        // let v = [
        //     array![[1, 1, 1]],
        //     array![[2, 2, 2]],
        //     array![[3, 3, 3]],
        //     array![[4, 4, 4]],
        // ];
        // let v_iter = v.into_iter();
        // // The `iter` method produces an `Iterator` over an array/slice.
        // for i in v_iter {
        //     println!("{:?}", i);
        // }

        // let a = array![[[1, 1, 1]], [[2, 2, 2]], [[3, 3, 3]], [[4, 4, 4]]];
        // for i in a.outer_iter() {
        //     println!("{:?}", i) // iterate through first dimension
        // }

        let ar1: ArrayBase<OwnedRepr<i32>, _> = array![[1; 5]];
        println!("{:?}", ar1); // [1, 1, 1, 1, 1]
                               // for i in ar1.into_iter() {
                               //     println!("{:?}", i) // iterate through first dimension
                               // }
        let c = ar1.into_shape([5, 1]).unwrap();
        println!("{:?}", c);
    }

    #[test]
    fn stream_qkt_test() {
        const HEAD_DIM: usize = 16;
        const HEAD_DIM_I32: i32 = 16;
        const SEQ_LEN: usize = 2;

        // We will use seq length of 5 for now. So, conservatively keep the FIFO (Channel) size as 5.
        let chan_size = SEQ_LEN;

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
            in1: in1_receiver,
            in2: in2_receiver,
            out: out_sender,
        };

        let stream_qkt = QKT::new(data);

        // Create the Iterators for Generators
        let in1_iter = || {
            [
                array![[1; HEAD_DIM]],
                array![[2; HEAD_DIM]],
                array![[3; HEAD_DIM]],
                array![[4; HEAD_DIM]],
                array![[5; HEAD_DIM]],
            ]
            .into_iter()
        };
        let in2_iter = || {
            [
                array![[1; HEAD_DIM]].into_shape([HEAD_DIM, 1]).unwrap(),
                array![[1; HEAD_DIM]].into_shape([HEAD_DIM, 1]).unwrap(),
                array![[1; HEAD_DIM]].into_shape([HEAD_DIM, 1]).unwrap(),
                array![[1; HEAD_DIM]].into_shape([HEAD_DIM, 1]).unwrap(),
                array![[1; HEAD_DIM]].into_shape([HEAD_DIM, 1]).unwrap(),
            ]
            .into_iter()
        };

        // Create the Iterators for Checkers
        let out_iter = || {
            [
                array![[1 * HEAD_DIM_I32]],
                array![[2 * HEAD_DIM_I32]],
                array![[3 * HEAD_DIM_I32]],
                array![[4 * HEAD_DIM_I32]],
                array![[5 * HEAD_DIM_I32]],
            ]
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
