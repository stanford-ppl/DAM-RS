use std::collections::VecDeque;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dam_macros::{identifiable, time_managed};
use dam_rs::channel::*;
use dam_rs::context::checker_context::CheckerContext;
use dam_rs::context::consumer_context::ConsumerContext;
use dam_rs::context::generator_context::GeneratorContext;
use dam_rs::context::Context;
use dam_rs::simulation::Program;
use dam_rs::types::{Cleanable, DAMType};

#[identifiable]
#[time_managed]
struct MergeUnit<T: DAMType> {
    input_a: Receiver<T>,
    input_b: Receiver<T>,
    output: Sender<T>,
}

impl<T: DAMType + std::cmp::Ord> Context for MergeUnit<T> {
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            let a = self.input_a.peek_next(&mut self.time);
            let b = self.input_b.peek_next(&mut self.time);
            match (a, b) {
                (DequeueResult::Something(ce_a), DequeueResult::Something(ce_b)) => {
                    let min = std::cmp::min(ce_a.data.clone(), ce_b.data.clone());
                    if ce_a.data == min {
                        self.input_a.dequeue(&mut self.time);
                    }
                    if ce_b.data == min {
                        self.input_b.dequeue(&mut self.time);
                    }
                    let time = self.time.tick() + 1;
                    self.output
                        .enqueue(&mut self.time, ChannelElement::new(time, min))
                        .unwrap();
                }
                (DequeueResult::Something(ce_a), DequeueResult::Closed) => {
                    self.input_a.dequeue(&mut self.time);
                    self.output.enqueue(&mut self.time, ce_a).unwrap();
                }
                (DequeueResult::Closed, DequeueResult::Something(ce_b)) => {
                    self.input_b.dequeue(&mut self.time);
                    self.output.enqueue(&mut self.time, ce_b).unwrap();
                }
                (DequeueResult::Closed, DequeueResult::Closed) => return,
                _ => panic!(),
            }
            self.time.incr_cycles(1);
        }
    }

    fn cleanup(&mut self) {
        self.input_a.cleanup();
        self.input_b.cleanup();
        self.output.cleanup();
        self.time.cleanup();
    }
}

impl<T: DAMType + Ord> MergeUnit<T> {
    pub fn new(a: Receiver<T>, b: Receiver<T>, out: Sender<T>) -> Self {
        let mu = Self {
            input_a: a,
            input_b: b,
            output: out,
            identifier: Default::default(),
            time: Default::default(),
        };
        mu.input_a.attach_receiver(&mu);
        mu.input_b.attach_receiver(&mu);
        mu.output.attach_sender(&mu);
        mu
    }
}

pub fn merge_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("SST_Merge");
    let test_size: i32 = 1 << 24;
    const CHAN_SIZE: usize = 32;

    for power in 0..4 {
        let copies = 1 << power;
        group.sample_size(10);
        group.bench_with_input(
            BenchmarkId::from_parameter(copies),
            &copies,
            |b, &copies| {
                b.iter_with_large_drop(|| {
                    // two-stage PCU on scalars, with the third stage a no-op.
                    let mut parent = Program::default();

                    for _ in 0..copies {
                        let (a_send, a_recv) = parent.bounded(CHAN_SIZE);
                        let (b_send, b_recv) = parent.bounded(CHAN_SIZE);
                        let (c_send, c_recv) = parent.bounded(CHAN_SIZE);
                        let gen_a = GeneratorContext::new(|| (0..test_size), a_send);
                        let gen_b = GeneratorContext::new(|| 0..test_size, b_send);
                        let merge = MergeUnit::new(a_recv, b_recv, c_send);
                        let checker = CheckerContext::new(|| 0..test_size, c_recv);
                        parent.add_child(gen_a);
                        parent.add_child(gen_b);
                        parent.add_child(merge);
                        parent.add_child(checker);
                    }

                    parent.set_inference(true);
                    parent.set_mode(dam_rs::simulation::RunMode::FIFO);
                    parent.init();
                    parent.run();
                    parent
                })
            },
        );
    }
    group.finish();
}

#[identifiable]
#[time_managed]
struct AddUnit<T: DAMType> {
    input_a: Receiver<T>,
    input_b: Receiver<T>,
    output: Sender<T>,
}

impl<T: DAMType> Context for AddUnit<T>
where
    T: std::ops::Add<T, Output = T>,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            let a = self.input_a.dequeue(&mut self.time);
            let b = self.input_b.dequeue(&mut self.time);
            match (a, b) {
                (DequeueResult::Something(ce_a), DequeueResult::Something(ce_b)) => {
                    let time = self.time.tick() + 1;
                    self.output
                        .enqueue(
                            &mut self.time,
                            ChannelElement::new(time, ce_a.data + ce_b.data),
                        )
                        .unwrap();
                }
                (DequeueResult::Closed, DequeueResult::Closed) => return,
                _ => panic!(),
            }
            self.time.incr_cycles(1);
        }
    }

    fn cleanup(&mut self) {
        self.input_a.cleanup();
        self.input_b.cleanup();
        self.output.cleanup();
        self.time.cleanup();
    }
}

impl<T: DAMType> AddUnit<T>
where
    T: std::ops::Add<T, Output = T>,
{
    pub fn new(a: Receiver<T>, b: Receiver<T>, out: Sender<T>) -> Self {
        let mu = Self {
            input_a: a,
            input_b: b,
            output: out,
            identifier: Default::default(),
            time: Default::default(),
        };
        mu.input_a.attach_receiver(&mu);
        mu.input_b.attach_receiver(&mu);
        mu.output.attach_sender(&mu);
        mu
    }
}

pub fn add_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("SST_Add");
    let test_size: i32 = 1 << 18;
    const CHAN_SIZE: usize = 32;

    for power in 10..11 {
        let width = 1 << power;
        group.sample_size(10);
        group.bench_with_input(BenchmarkId::from_parameter(width), &width, |b, &width| {
            b.iter_with_large_drop(|| {
                // two-stage PCU on scalars, with the third stage a no-op.
                let mut parent = Program::default();

                let mut cur_inputs: VecDeque<Receiver<i32>> = VecDeque::with_capacity(width);
                for _ in 0..width {
                    let (send, recv) = parent.bounded(CHAN_SIZE);
                    // let chan_id = send.id();
                    let gen = GeneratorContext::new(|| (0..test_size), send);
                    parent.add_child(gen);
                    cur_inputs.push_back(recv);

                    // println!("New Channel: {chan_id:?}")
                }

                loop {
                    let a = cur_inputs.pop_front().unwrap();
                    let b = cur_inputs.pop_front().unwrap();
                    let (c_send, c_recv) = parent.bounded(CHAN_SIZE);
                    // let chan_id = c_send.id();
                    let merge = AddUnit::new(a, b, c_send);
                    parent.add_child(merge);
                    cur_inputs.push_back(c_recv);

                    // println!("New Channel: {chan_id:?}");
                    if cur_inputs.len() == 1 {
                        break;
                    }
                }

                let last = cur_inputs.pop_front().unwrap();
                let cap = ConsumerContext::new(last);
                parent.add_child(cap);

                parent.set_inference(true);
                parent.set_mode(dam_rs::simulation::RunMode::FIFO);
                parent.init();
                parent.run();
                parent
            })
        });
    }
    group.finish();
}

criterion_group!(sst_benches, merge_benchmark, add_benchmark);
criterion_main!(sst_benches);
