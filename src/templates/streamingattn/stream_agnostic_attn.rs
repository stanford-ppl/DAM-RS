use dam_core::identifier::Identifier;
use dam_core::TimeManager;
use dam_macros::{cleanup, identifiable, time_managed};

use crate::{
    channel::{
        utils::{dequeue, enqueue, peek_next},
        ChannelElement, Receiver, Sender,
    },
    context::Context,
    templates::streamingattn::stream_reduce::MinMax,
    types::{Cleanable, DAMType},
};

pub struct IncrMaxData<A: Clone> {
    pub in_stream: Receiver<A>,
    pub delta_out_stream: Vec<Sender<A>>,
    pub curr_out_stream: Vec<Sender<A>>,
    pub latency: u64,
    pub init_inverval: u64,
    pub inner_loop_bound: u64,
    pub outer_loop_bound: u64,
}

impl<A: DAMType> Cleanable for IncrMaxData<A> {
    fn cleanup(&mut self) {
        self.in_stream.cleanup();
        for i in self.delta_out_stream.iter_mut() {
            i.cleanup();
        }
        for i in self.curr_out_stream.iter_mut() {
            i.cleanup();
        }
    }
}

#[time_managed]
#[identifiable]
pub struct IncrMax<A: Clone> {
    incr_data: IncrMaxData<A>,
}

impl<A: DAMType> IncrMax<A>
where
    IncrMax<A>: Context,
{
    pub fn new(incr_data: IncrMaxData<A>) -> Self {
        let incr_max = IncrMax {
            incr_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (incr_max.incr_data.in_stream).attach_receiver(&incr_max);
        for i in incr_max.incr_data.delta_out_stream.iter() {
            i.attach_sender(&incr_max);
        }
        for i in incr_max.incr_data.curr_out_stream.iter() {
            i.attach_sender(&incr_max);
        }

        incr_max
    }
}

impl<A> Context for IncrMax<A>
where
    A: DAMType + num::Float + MinMax + Copy,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        for _i in 0..self.incr_data.outer_loop_bound {
            let mut temp_res = A::get_min_val();
            for _j in 0..self.incr_data.inner_loop_bound {
                let in_deq = dequeue(&mut self.time, &mut self.incr_data.in_stream);
                match in_deq {
                    Ok(in_elem) => {
                        // First Iteration
                        let in_data = in_elem.data;
                        let new_max = temp_res.get_max(in_data);
                        let delta = (temp_res - new_max).exp();
                        let curr = (in_data - new_max).exp();
                        temp_res = new_max;

                        let curr_time = self.time.tick();
                        for mut k in self.incr_data.delta_out_stream.iter_mut() {
                            enqueue(
                                &mut self.time,
                                &mut k,
                                ChannelElement::new(
                                    curr_time + self.incr_data.latency,
                                    delta.clone(),
                                ),
                            )
                            .unwrap();
                        }
                        for mut k in self.incr_data.curr_out_stream.iter_mut() {
                            enqueue(
                                &mut self.time,
                                &mut k,
                                ChannelElement::new(
                                    curr_time + self.incr_data.latency,
                                    curr.clone(),
                                ),
                            )
                            .unwrap();
                        }

                        self.time.incr_cycles(self.incr_data.init_inverval);
                        // initiation interval
                    }
                    _ => {
                        panic!("Reached unhandled case");
                    }
                }
            }
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.incr_data.cleanup();
        self.time.cleanup();
    }
}

pub struct IncrSumData<A: Clone> {
    pub in_delta_stream: Receiver<A>,
    pub in_curr_stream: Receiver<A>,
    pub out_stream: Sender<A>,
    pub latency: u64,
    pub init_inverval: u64,
    pub inner_loop_bound: u64,
    pub outer_loop_bound: u64,
}

impl<A: DAMType> Cleanable for IncrSumData<A> {
    fn cleanup(&mut self) {
        self.in_delta_stream.cleanup();
        self.in_curr_stream.cleanup();
        self.out_stream.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct IncrSum<A: Clone> {
    incr_data: IncrSumData<A>,
}

impl<A: DAMType> IncrSum<A>
where
    IncrSum<A>: Context,
{
    pub fn new(incr_data: IncrSumData<A>) -> Self {
        let incr_sum = IncrSum {
            incr_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (incr_sum.incr_data.in_delta_stream).attach_receiver(&incr_sum);
        (incr_sum.incr_data.in_curr_stream).attach_receiver(&incr_sum);
        (incr_sum.incr_data.out_stream).attach_sender(&incr_sum);

        incr_sum
    }
}

impl<A> Context for IncrSum<A>
where
    A: DAMType + num::Num + MinMax + Copy,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        for _i in 0..self.incr_data.outer_loop_bound {
            let mut temp_res = A::get_zero();
            for j in 0..self.incr_data.inner_loop_bound {
                let _ = peek_next(&mut self.time, &mut self.incr_data.in_delta_stream);
                let _ = peek_next(&mut self.time, &mut self.incr_data.in_curr_stream);
                let in_delta_deq = dequeue(&mut self.time, &mut self.incr_data.in_delta_stream);
                let in_curr_deq = dequeue(&mut self.time, &mut self.incr_data.in_curr_stream);
                match (in_delta_deq, in_curr_deq) {
                    (Ok(in_delta), Ok(in_curr)) => {
                        // First Iteration
                        let in_delta_data = in_delta.data;
                        let in_curr_data = in_curr.data;
                        let new_sum = temp_res * in_delta_data + in_curr_data;
                        temp_res = new_sum;

                        if j == self.incr_data.inner_loop_bound - 1 {
                            let curr_time = self.time.tick();
                            enqueue(
                                &mut self.time,
                                &mut self.incr_data.out_stream,
                                ChannelElement::new(curr_time + self.incr_data.latency, temp_res),
                            )
                            .unwrap();
                        }

                        self.time.incr_cycles(self.incr_data.init_inverval);
                        // initiation interval
                    }
                    (_, _) => {
                        panic!("Reached unhandled case");
                    }
                }
            }
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.incr_data.cleanup();
        self.time.cleanup();
    }
}

pub struct IncrOutPData<A: Clone> {
    pub in_delta_stream: Receiver<A>,
    pub in_curr_stream: Receiver<A>,
    pub in_v_stream: Receiver<A>, // should be an vector, but we assume d=1 for simplicity
    pub out_stream: Sender<A>,
    pub latency: u64,
    pub init_inverval: u64,
    pub inner_loop_bound: u64,
    pub outer_loop_bound: u64,
}

impl<A: DAMType> Cleanable for IncrOutPData<A> {
    fn cleanup(&mut self) {
        self.in_delta_stream.cleanup();
        self.in_curr_stream.cleanup();
        self.in_v_stream.cleanup();
        self.out_stream.cleanup();
    }
}

#[time_managed]
#[identifiable]
pub struct IncrOutP<A: Clone> {
    incr_data: IncrOutPData<A>,
}

impl<A: DAMType> IncrOutP<A>
where
    IncrOutP<A>: Context,
{
    pub fn new(incr_data: IncrOutPData<A>) -> Self {
        let incr_outer_p = IncrOutP {
            incr_data,
            time: TimeManager::default(),
            identifier: Identifier::new(),
        };
        (incr_outer_p.incr_data.in_delta_stream).attach_receiver(&incr_outer_p);
        (incr_outer_p.incr_data.in_curr_stream).attach_receiver(&incr_outer_p);
        (incr_outer_p.incr_data.in_v_stream).attach_receiver(&incr_outer_p);
        (incr_outer_p.incr_data.out_stream).attach_sender(&incr_outer_p);

        incr_outer_p
    }
}

impl<A> Context for IncrOutP<A>
where
    A: DAMType + num::Num + MinMax + Copy,
{
    fn init(&mut self) {}

    fn run(&mut self) -> () {
        for _i in 0..self.incr_data.outer_loop_bound {
            let mut temp_res = A::get_zero();
            for j in 0..self.incr_data.inner_loop_bound {
                let _ = peek_next(&mut self.time, &mut self.incr_data.in_delta_stream);
                let _ = peek_next(&mut self.time, &mut self.incr_data.in_curr_stream);
                let _ = peek_next(&mut self.time, &mut self.incr_data.in_v_stream);
                let in_delta_deq = dequeue(&mut self.time, &mut self.incr_data.in_delta_stream);
                let in_curr_deq = dequeue(&mut self.time, &mut self.incr_data.in_curr_stream);
                let in_v_deq = dequeue(&mut self.time, &mut self.incr_data.in_v_stream);
                match (in_delta_deq, in_curr_deq, in_v_deq) {
                    (Ok(in_delta), Ok(in_curr), Ok(in_v)) => {
                        // First Iteration
                        let in_delta_data = in_delta.data;
                        let in_curr_data = in_curr.data;
                        let in_v_data = in_v.data;
                        let new_sum = temp_res * in_delta_data + in_curr_data * in_v_data;
                        temp_res = new_sum;

                        if j == self.incr_data.inner_loop_bound - 1 {
                            let curr_time = self.time.tick();
                            enqueue(
                                &mut self.time,
                                &mut self.incr_data.out_stream,
                                ChannelElement::new(curr_time + self.incr_data.latency, temp_res),
                            )
                            .unwrap();
                        }

                        self.time.incr_cycles(self.incr_data.init_inverval);
                        // initiation interval
                    }
                    (_, _, _) => {
                        panic!("Reached unhandled case");
                    }
                }
            }
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.incr_data.cleanup();
        self.time.cleanup();
    }
}
