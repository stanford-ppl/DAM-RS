use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use dam_rs::templates::ops::*;
use dam_rs::utility_contexts::*;
use dam_rs::{simulation::*, templates::pcu::*};

pub fn pcu_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("PCU_MulAdd");
    for power in 0..16 {
        let size = 1 << power;
        group.throughput(criterion::Throughput::Elements(size));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_with_large_drop(|| {
                // two-stage PCU on scalars, with the third stage a no-op.
                let mut parent = ProgramBuilder::default();

                const CHAN_SIZE: usize = 8;
                let ingress_op = PCU::READ_ALL_INPUTS;
                let egress_op = PCU::WRITE_ALL_RESULTS;

                let mut pcu = PCU::<u64>::new(
                    PCUConfig::<u64> {
                        pipeline_depth: 3,
                        num_registers: 3,
                        counter: None,
                    },
                    ingress_op,
                    egress_op,
                );

                pcu.push_stage(PipelineStage {
                    op: ALUMulOp(),
                    forward: vec![(2, 1)],
                    prev_register_ids: vec![0, 1],
                    next_register_ids: vec![],
                    output_register_ids: vec![0],
                });

                pcu.push_stage(PipelineStage {
                    op: ALUAddOp(),
                    forward: vec![],
                    prev_register_ids: vec![0, 1],
                    next_register_ids: vec![],
                    output_register_ids: vec![0],
                });

                let (arg1_send, arg1_recv) = parent.bounded(CHAN_SIZE);
                let (arg2_send, arg2_recv) = parent.bounded(CHAN_SIZE);
                let (arg3_send, arg3_recv) = parent.bounded(CHAN_SIZE);
                let (pcu_out_send, pcu_out_recv) = parent.bounded(CHAN_SIZE);

                pcu.add_input_channel(arg1_recv);
                pcu.add_input_channel(arg2_recv);
                pcu.add_input_channel(arg3_recv);
                pcu.add_output_channel(pcu_out_send);

                let gen1 = GeneratorContext::new(|| (0..size), arg1_send);
                let gen2 = GeneratorContext::new(|| ((size)..(2 * size)), arg2_send);
                let gen3 = GeneratorContext::new(|| ((2 * size)..(3 * size)), arg3_send);
                let checker = CheckerContext::new(
                    || (0..size).map(|x| x * (x + size) + (x + size * 2)),
                    pcu_out_recv,
                );

                parent.add_child(gen1);
                parent.add_child(gen2);
                parent.add_child(gen3);
                parent.add_child(pcu);
                parent.add_child(checker);
                parent
                    .initialize(InitializationOptions::default())
                    .unwrap()
                    .run(RunMode::Simple);
            })
        });
    }
    group.finish();
}

criterion_group!(template_benches, pcu_benchmark);
criterion_main!(template_benches);
