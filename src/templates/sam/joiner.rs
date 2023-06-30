use crate::{
    channel::{
        utils::{dequeue, enqueue},
        ChannelElement, Receiver, Sender,
    },
    context::{view::TimeManager, Context},
    types::{Cleanable, DAMType},
};

use super::primitive::Token;

pub struct IntersectData<ValType, StopType> {
    in_crd1: Receiver<Token<ValType, StopType>>,
    in_ref1: Receiver<Token<ValType, StopType>>,
    in_crd2: Receiver<Token<ValType, StopType>>,
    in_ref2: Receiver<Token<ValType, StopType>>,
    out_ref1: Sender<Token<ValType, StopType>>,
    out_ref2: Sender<Token<ValType, StopType>>,
    out_crd: Sender<Token<ValType, StopType>>,
}

impl<ValType: DAMType, StopType: DAMType> Cleanable for IntersectData<ValType, StopType> {
    fn cleanup(&mut self) {
        self.in_crd1.cleanup();
        self.in_ref1.cleanup();
        self.in_crd2.cleanup();
        self.in_ref2.cleanup();
        self.out_ref1.cleanup();
        self.out_ref2.cleanup();
        self.out_crd.cleanup();
    }
}

pub struct Intersect<ValType, StopType> {
    intersect_data: IntersectData<ValType, StopType>,
    // meta_dim: ValType,
    time: TimeManager,
}

impl<ValType: DAMType, StopType: DAMType> Intersect<ValType, StopType>
where
    Intersect<ValType, StopType>: Context,
{
    pub fn new(intersect_data: IntersectData<ValType, StopType>) -> Self {
        let int = Intersect {
            intersect_data,
            time: TimeManager::default(),
        };
        (int.intersect_data.in_crd1).attach_receiver(&int);
        (int.intersect_data.in_ref1).attach_receiver(&int);
        (int.intersect_data.in_crd2).attach_receiver(&int);
        (int.intersect_data.in_ref2).attach_receiver(&int);
        (int.intersect_data.out_ref1).attach_sender(&int);
        (int.intersect_data.out_ref2).attach_sender(&int);
        (int.intersect_data.out_crd).attach_sender(&int);

        int
    }
}

impl<ValType, StopType> Context for Intersect<ValType, StopType>
where
    ValType: DAMType
        + std::ops::AddAssign<u32>
        + std::ops::AddAssign<ValType>
        + std::ops::Mul<ValType, Output = ValType>
        + std::ops::Add<ValType, Output = ValType>
        + std::cmp::PartialOrd<ValType>,
    StopType:
        DAMType + std::ops::Add<u32, Output = StopType> + std::ops::Sub<u32, Output = StopType>,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            // println!("seg: {:?}", self.seg_arr);
            // println!("crd: {:?}", self.crd_arr);
            self.time.incr_cycles(1);
        }
    }

    fn cleanup(&mut self) {
        self.intersect_data.cleanup();
        self.time.cleanup();
    }

    fn view(&self) -> Box<dyn crate::context::ContextView> {
        Box::new(self.time.view())
    }
}
