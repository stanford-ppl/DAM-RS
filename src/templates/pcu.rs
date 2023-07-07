use std::sync::{Arc, Mutex};

use crate::{
    channel::{
        utils::{dequeue, enqueue},
        ChannelElement, Receiver, Sender,
    },
    context::{
        view::{TimeManager, TimeView},
        Context,
    },
    time::Time,
    types::DAMType,
};

use super::ops::{ALUOp, PipelineRegister};

#[derive(Debug)]
pub struct PCUConfig {
    pub pipeline_depth: usize,
    pub num_registers: usize,
}

#[derive(Debug)]
pub struct PipelineStage<ET: DAMType> {
    pub op: ALUOp<ET>,
    pub forward: Vec<(usize, usize)>,
    pub prev_register_ids: Vec<usize>,
    pub next_register_ids: Vec<usize>,
    pub output_register_ids: Vec<usize>,
}

impl<ET: DAMType> PipelineStage<ET> {
    pub fn run(
        &self,
        prev_registers: &[PipelineRegister<ET>],
        next_registers: &[PipelineRegister<ET>],
    ) -> Vec<PipelineRegister<ET>> {
        let mapped_inputs: Vec<PipelineRegister<ET>> = self
            .prev_register_ids
            .iter()
            .map(|ind| prev_registers[*ind].clone())
            .collect();
        let mapped_outputs: Vec<PipelineRegister<ET>> = self
            .next_register_ids
            .iter()
            .map(|ind| next_registers[*ind].clone())
            .collect();
        let func_outputs = (self.op.func)(&mapped_inputs, &mapped_outputs);

        // Copy the next registers into a new copy of registers
        let mut outputs = next_registers.to_vec();

        // Place the outputs of the func into the appropriate registers
        self.output_register_ids
            .iter()
            .enumerate()
            .for_each(|(src, dst)| {
                outputs[*dst] = func_outputs[src].clone();
            });

        // Forward the appropriate prev_registers into the new next_registers
        self.forward.iter().for_each(|(src, dst)| {
            outputs[*dst] = prev_registers[*src].clone();
        });

        outputs
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PipelineState {
    // when did this value arrive?
    time: Time,
    valid: bool,
}

type InputChannelsType<ElementType> = Arc<Mutex<Vec<Receiver<ElementType>>>>;
type OutputChannelsType<ElementType> = Arc<Mutex<Vec<Sender<ElementType>>>>;
type IngressOpType<ElementType> = fn(
    &mut InputChannelsType<ElementType>,
    &mut Vec<PipelineRegister<ElementType>>,
    &mut TimeManager,
) -> bool;

type EgressOpType<ElementType> = fn(
    &mut OutputChannelsType<ElementType>,
    &Vec<PipelineRegister<ElementType>>,
    Time,
    &mut TimeManager,
);

pub struct PCU<ElementType: DAMType> {
    time: TimeManager,
    configuration: PCUConfig,
    registers: Vec<Vec<PipelineRegister<ElementType>>>,

    // The operation to run, and the pipeline registers to forward
    stages: Vec<PipelineStage<ElementType>>,

    input_channels: InputChannelsType<ElementType>,
    output_channels: OutputChannelsType<ElementType>,

    ingress_op: IngressOpType<ElementType>,

    egress_op: EgressOpType<ElementType>,
}

impl<ElementType: DAMType> PCU<ElementType> {
    pub fn new(
        configuration: PCUConfig,
        ingress_op: IngressOpType<ElementType>,
        egress_op: EgressOpType<ElementType>,
    ) -> PCU<ElementType> {
        let pipe_depth = configuration.pipeline_depth;
        let mut registers = Vec::<Vec<PipelineRegister<ElementType>>>::new();
        registers.resize_with(pipe_depth + 1 /* plus one for egress */, || {
            let mut pipe_regs = Vec::<PipelineRegister<ElementType>>::new();
            pipe_regs.resize_with(configuration.num_registers, Default::default);
            pipe_regs
        });
        PCU::<ElementType> {
            time: TimeManager::default(),
            configuration,
            registers,
            stages: Vec::with_capacity(pipe_depth),
            input_channels: Arc::new(Mutex::new(vec![])),
            output_channels: Arc::new(Mutex::new(vec![])),
            ingress_op,
            egress_op,
        }
    }

    pub const READ_ALL_INPUTS: IngressOpType<ElementType> = |ics, regs, time| {
        let mut reads: Vec<_> = ics
            .lock()
            .unwrap()
            .iter_mut()
            .map(|recv| dequeue(time, recv))
            .collect();

        for (ind, read) in reads.iter_mut().enumerate() {
            if let Err(_) = read {
                return false;
            }
            regs[ind].data = read.as_ref().unwrap().data.clone();
        }

        return true;
    };

    pub const WRITE_ALL_RESULTS: EgressOpType<ElementType> = |ocs, regs, out_time, manager| {
        ocs.lock()
            .unwrap()
            .iter_mut()
            .enumerate()
            .for_each(|(ind, out_chan)| {
                enqueue(
                    manager,
                    out_chan,
                    ChannelElement {
                        time: out_time,
                        data: regs[ind].data.clone(),
                    },
                )
                .unwrap();
            });
    };

    pub fn push_stage(&mut self, stage: PipelineStage<ElementType>) {
        self.stages.push(stage);
        assert!(self.stages.len() <= self.configuration.pipeline_depth);
    }

    pub fn add_input_channel(&mut self, recv: Receiver<ElementType>) {
        recv.attach_receiver(self);
        self.input_channels.lock().unwrap().push(recv);
    }

    pub fn add_output_channel(&mut self, send: Sender<ElementType>) {
        send.attach_sender(self);
        self.output_channels.lock().unwrap().push(send);
    }
}

impl<ElementType: DAMType> Context for PCU<ElementType> {
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            // Run the entire pipeline from front to back
            if !(self.ingress_op)(
                &mut self.input_channels,
                &mut self.registers[0],
                &mut self.time,
            ) {
                return;
            }

            for stage_index in 0..self.configuration.pipeline_depth {
                match self.stages.get(stage_index) {
                    Some(cur_stage) => {
                        let prev_regs = &self.registers[stage_index];
                        let next_regs = &self.registers[stage_index + 1];
                        let new_regs = cur_stage.run(&prev_regs, &next_regs);
                        self.registers[stage_index + 1] = new_regs;
                    }
                    None => self.registers[stage_index + 1] = self.registers[stage_index].clone(),
                }
            }

            let latency = u64::try_from(self.configuration.pipeline_depth).unwrap();

            // Need to provide a notion of time as to when this result is ready.
            (self.egress_op)(
                &mut self.output_channels,
                &mut self.registers[self.configuration.pipeline_depth],
                self.time.tick() + Time::new(latency),
                &mut self.time,
            );
            self.time.incr_cycles(1);
        }
    }

    fn cleanup(&mut self) {
        self.input_channels
            .lock()
            .unwrap()
            .iter_mut()
            .for_each(|chan| {
                chan.close();
            });
        self.time.cleanup();
    }

    fn view(&self) -> TimeView {
        self.time.view().into()
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        channel::bounded,
        context::{
            checker_context::CheckerContext, generator_context::GeneratorContext,
            parent::BasicParentContext, Context, ParentContext,
        },
        templates::ops::*,
    };

    use super::PCU;

    #[test]
    fn pcu_test() {
        // two-stage PCU on scalars, with the third stage a no-op.

        const CHAN_SIZE: usize = 8;
        let ingress_op = PCU::<u16>::READ_ALL_INPUTS;
        let egress_op = PCU::<u16>::WRITE_ALL_RESULTS;

        let mut pcu = PCU::<u16>::new(
            super::PCUConfig {
                pipeline_depth: 3,
                num_registers: 3,
            },
            ingress_op,
            egress_op,
        );

        pcu.push_stage(super::PipelineStage {
            op: ALUMulOp::<u16>(),
            forward: vec![(2, 1)],
            prev_register_ids: vec![0, 1],
            next_register_ids: vec![],
            output_register_ids: vec![0],
        });

        pcu.push_stage(super::PipelineStage {
            op: ALUAddOp::<u16>(),
            forward: vec![],
            prev_register_ids: vec![0, 1],
            next_register_ids: vec![],
            output_register_ids: vec![0],
        });

        let (arg1_send, arg1_recv) = bounded::<u16>(CHAN_SIZE);
        let (arg2_send, arg2_recv) = bounded::<u16>(CHAN_SIZE);
        let (arg3_send, arg3_recv) = bounded::<u16>(CHAN_SIZE);
        let (pcu_out_send, pcu_out_recv) = bounded::<u16>(CHAN_SIZE);

        pcu.add_input_channel(arg1_recv);
        pcu.add_input_channel(arg2_recv);
        pcu.add_input_channel(arg3_recv);
        pcu.add_output_channel(pcu_out_send);

        let mut gen1 = GeneratorContext::new(|| (0u16..32), arg1_send);
        let mut gen2 = GeneratorContext::new(|| (33u16..64), arg2_send);
        let mut gen3 = GeneratorContext::new(|| (65u16..96), arg3_send);
        let mut checker = CheckerContext::new(
            || {
                (0u16..32)
                    .zip(33u16..64)
                    .zip(65u16..96)
                    .map(|((a, b), c)| (a * b) + c)
            },
            pcu_out_recv,
        );

        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut gen2);
        parent.add_child(&mut gen3);
        parent.add_child(&mut pcu);
        parent.add_child(&mut checker);
        parent.init();
        parent.run();
        parent.cleanup();
    }
}
