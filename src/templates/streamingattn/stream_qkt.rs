use core::fmt::Debug;
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
    pub in1: Receiver<ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>>,
    pub in2: Receiver<ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>>,
    pub out: Sender<ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>>,
}

impl<A: Clone> Cleanable for QKTData<A>
where
    ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>: DAMType,
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
}

impl<A: Clone> QKT<A>
where
    QKT<A>: Context,
    ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>: DAMType,
{
    pub fn new(qkt_data: QKTData<A>) -> Self {
        let qkt = QKT {
            qkt_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (qkt.qkt_data.in1).attach_receiver(&qkt);
        (qkt.qkt_data.in2).attach_receiver(&qkt);
        (qkt.qkt_data.out).attach_sender(&qkt);

        qkt
    }
}

impl<A: Clone> Context for QKT<A>
where
    ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>: DAMType,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {}

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.qkt_data.cleanup();
        self.time.cleanup();
    }
}
