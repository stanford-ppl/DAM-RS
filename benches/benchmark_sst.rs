use std::collections::VecDeque;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dam::channel::*;
use dam::context::Context;
use dam::context_tools::*;
use dam::simulation::*;
use dam::types::DAMType;
use dam::utility_contexts::*;

#[context_macro]
struct MergeUnit<T: DAMType> {
    input_a: Receiver<T>,
    input_b: Receiver<T>,
    output: Sender<T>,
}

impl<T: DAMType + std::cmp::Ord> Context for MergeUnit<T> {
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            let a = self.input_a.peek_next(&self.time);
            let b = self.input_b.peek_next(&self.time);
            match (a, b) {
                (Ok(ce_a), Ok(ce_b)) => {
                    let min = std::cmp::min(ce_a.data.clone(), ce_b.data.clone());
                    if ce_a.data == min {
                        self.input_a.dequeue(&self.time).unwrap();
                    }
                    if ce_b.data == min {
                        self.input_b.dequeue(&self.time).unwrap();
                    }
                    let time = self.time.tick() + 1;
                    self.output
                        .enqueue(&self.time, ChannelElement::new(time, min))
                        .unwrap();
                }
                (Ok(ce_a), Err(_)) => {
                    self.input_a.dequeue(&self.time).unwrap();
                    self.output.enqueue(&self.time, ce_a).unwrap();
                }
                (Err(_), Ok(ce_b)) => {
                    self.input_b.dequeue(&self.time).unwrap();
                    self.output.enqueue(&self.time, ce_b).unwrap();
                }
                (Err(_), Err(_)) => return,
            }
            self.time.incr_cycles(1);
        }
    }
}

impl<T: DAMType + Ord> MergeUnit<T> {
    pub fn new(a: Receiver<T>, b: Receiver<T>, out: Sender<T>) -> Self {
        let mu = Self {
            input_a: a,
            input_b: b,
            output: out,
            context_info: Default::default(),
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
                    let mut parent = ProgramBuilder::default();

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

                    parent
                        .initialize(
                            InitializationOptionsBuilder::default()
                                .run_flavor_inference(true)
                                .build()
                                .unwrap(),
                        )
                        .unwrap()
                        .run(RunOptionsBuilder::default().build().unwrap());
                })
            },
        );
    }
    group.finish();
}

#[context_macro]
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
            let a = self.input_a.dequeue(&self.time);
            let b = self.input_b.dequeue(&self.time);
            match (a, b) {
                (Ok(ce_a), Ok(ce_b)) => {
                    let time = self.time.tick() + 1;
                    self.output
                        .enqueue(&self.time, ChannelElement::new(time, ce_a.data + ce_b.data))
                        .unwrap();
                }
                (Err(_), Err(_)) => return,
                _ => panic!(),
            }
            self.time.incr_cycles(1);
        }
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
            context_info: Default::default(),
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
                let mut parent = ProgramBuilder::default();

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

                parent
                    .initialize(
                        InitializationOptionsBuilder::default()
                            .run_flavor_inference(true)
                            .build()
                            .unwrap(),
                    )
                    .unwrap()
                    .run(RunOptionsBuilder::default().build().unwrap());
            })
        });
    }
    group.finish();
}

criterion_group!(sst_benches, merge_benchmark, add_benchmark);
criterion_main!(sst_benches);
