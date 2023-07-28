use core::fmt::Debug;
use dam_core::identifier::Identifier;
use dam_core::TimeManager;
use dam_macros::{cleanup, identifiable, time_managed};

use ndarray::{ArrayBase, Axis, Dimension, OwnedRepr};

use crate::{
    channel::{
        utils::{dequeue, enqueue, peek_next},
        ChannelElement, Receiver, Sender,
    },
    context::Context,
    types::{Cleanable, DAMType},
};

pub struct QKTData<A: Clone, D: Clone> {
    pub in1: Receiver<ArrayBase<OwnedRepr<A>, D>>,
    pub in2: Receiver<ArrayBase<OwnedRepr<A>, D>>,
    pub out: Sender<ArrayBase<OwnedRepr<A>, D>>,
}

impl<A: Clone, D: Clone> Cleanable for QKTData<A, D>
where
    ArrayBase<OwnedRepr<A>, D>: DAMType,
{
    fn cleanup(&mut self) {
        self.in1.cleanup();
        self.in2.cleanup();
        self.out.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct QKT<A: Clone, D: Clone> {
    qkt_data: QKTData<A, D>,
    //latency: dam_core::time::Time,
}

impl<A: Clone, D: Clone> QKT<A, D>
where
    QKT<A, D>: Context,
    ArrayBase<OwnedRepr<A>, D>: DAMType,
{
    pub fn new(qkt_data: QKTData<A, D>) -> Self {
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

impl<A, D> Context for QKT<A, D>
where
    ArrayBase<OwnedRepr<A>, D>: DAMType + std::ops::Mul<Output = ArrayBase<OwnedRepr<A>, D>>,
    D: Dimension<Smaller = D> + Clone + ndarray::RemoveAxis,
    A: Clone + num::Num + std::iter::Sum,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        loop {
            let in1_deq = peek_next(&mut self.time, &mut self.qkt_data.in1);
            let in2_deq = peek_next(&mut self.time, &mut self.qkt_data.in2);

            match (in1_deq, in2_deq) {
                (Ok(in1), Ok(in2)) => {
                    let mul_in12: ArrayBase<OwnedRepr<A>, D> = in1.data.clone() * in2.data;
                    // we clone as the multiplication consumes the first operand
                    let reduce_sum =
                        mul_in12.map_axis(Axis(0), |view| view.to_owned().into_iter().sum::<A>());
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

    use ndarray::{array, ArrayBase, Axis, Dim, OwnedRepr};

    #[test]
    fn test_ndarray_clone() {
        let array1 = array![[1, 2, 1], [3, 4, 5]];
        let array2 = array![[1, 2, 1], [3, 4, 5]];
        let array3 = array1.clone() * array2;
        //println!("{:?}", array1);
        println!("{:?}", array3);
        let reduction_arr3 = array3.map_axis(Axis(0), |view| view.iter().sum::<i32>());
        println!("{:?}", reduction_arr3);
    }
}
