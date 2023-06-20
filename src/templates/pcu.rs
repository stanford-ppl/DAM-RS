use std::sync::{Arc, Mutex};

use crate::{
    channel::{Receiver, Sender},
    context::{view::TimeManager, Context},
    time::Time,
    types::DAMType,
};

use super::ops::{ALUOp, PipelineRegister};

#[derive(Debug)]
pub struct PCUConfig {
    pipeline_depth: usize,
    num_registers: usize,
    initiation_interval: Time,
}

trait Operation<T: DAMType> {
    fn execute(
        previous: &[PipelineRegister<T>],
        next: &[PipelineRegister<T>],
    ) -> Vec<PipelineRegister<T>>;

    fn name() -> &'static str;
}

#[derive(Debug)]
pub struct PipelineStage<ET: Copy> {
    op: Arc<ALUOp<ET>>,
    forward: Vec<(usize, usize)>,
    prev_register_ids: Vec<usize>,
    next_register_ids: Vec<usize>,
    output_register_ids: Vec<usize>,
}

impl<ET: Copy> PipelineStage<ET> {
    pub fn run(
        &self,
        prev_registers: &[PipelineRegister<ET>],
        next_registers: &[PipelineRegister<ET>],
    ) -> Vec<PipelineRegister<ET>> {
        let mapped_inputs: Vec<PipelineRegister<ET>> = self
            .prev_register_ids
            .iter()
            .map(|ind| prev_registers[*ind])
            .collect();
        let mapped_outputs: Vec<PipelineRegister<ET>> = self
            .next_register_ids
            .iter()
            .map(|ind| next_registers[*ind])
            .collect();
        let func_outputs = (self.op.func)(&mapped_inputs, &mapped_outputs);

        // Copy the next registers into a new copy of registers
        let mut outputs = next_registers.to_vec();

        // Place the outputs of the func into the appropriate registers
        self.output_register_ids
            .iter()
            .enumerate()
            .for_each(|(src, dst)| {
                outputs[*dst] = func_outputs[src];
            });

        // Forward the appropriate prev_registers into the new next_registers
        self.forward.iter().for_each(|(src, dst)| {
            outputs[*src] = prev_registers[*dst];
        });
        outputs
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PipelineState {
    // when did this value arrive?
    time: Time,
}

pub struct PCU<ElementType: DAMType> {
    time: TimeManager,
    configuration: PCUConfig,
    registers: Vec<Vec<PipelineRegister<ElementType>>>,

    // The operation to run, and the pipeline registers to forward
    states: Vec<PipelineState>,
    stages: Vec<PipelineStage<ElementType>>,

    input_channels: Vec<Arc<Mutex<Receiver<ElementType>>>>,
    output_channels: Vec<Arc<Mutex<Sender<ElementType>>>>,

    ingress_op: Arc<
        Mutex<
            dyn Fn(
                    &mut Vec<Arc<Mutex<Receiver<ElementType>>>>,
                    &mut Vec<PipelineRegister<ElementType>>,
                    &mut TimeManager,
                ) -> bool
                + Sync
                + Send,
        >,
    >,

    egress_op: Arc<
        Mutex<
            dyn (Fn(
                    &mut Vec<Arc<Mutex<Sender<ElementType>>>>,
                    &Vec<PipelineRegister<ElementType>>,
                    Time,
                    &mut TimeManager,
                )) + Sync
                + Send,
        >,
    >,
}

impl<ElementType: DAMType> PCU<ElementType> {}

impl<ElementType: DAMType> Context for PCU<ElementType> {
    fn init(&mut self) {
        self.registers.resize_with(
            self.configuration.pipeline_depth + 1, /* plus one for  */
            || {
                let mut registers = Vec::<PipelineRegister<ElementType>>::new();
                registers.resize_with(self.configuration.num_registers, Default::default);
                registers
            },
        );
    }

    fn run(&mut self) {
        loop {
            // Run the entire pipeline from front to back
            if !(self.ingress_op.lock().unwrap())(
                &mut self.input_channels,
                &mut self.registers[0],
                &mut self.time,
            ) {
                return;
            }

            for stage_index in 0..self.configuration.pipeline_depth {
                let cur_stage = &self.stages[stage_index];
                let prev_regs = &self.registers[stage_index];
                let next_regs = &self.registers[stage_index + 1];
                let new_regs = cur_stage.run(&prev_regs, &next_regs);
                self.registers[stage_index + 1] = new_regs;
            }

            let latency = u64::try_from(self.configuration.pipeline_depth).unwrap();

            // Need to provide a notion of time as to when this result is ready.
            (self.egress_op.lock().unwrap())(
                &mut self.output_channels,
                &mut self.registers[self.configuration.pipeline_depth],
                self.time.tick() + Time::new(latency),
                &mut self.time,
            );
        }
    }

    fn cleanup(&mut self) {
        self.input_channels.iter_mut().for_each(|chan| {
            chan.lock().unwrap().close();
        });
    }

    fn view(&self) -> Box<dyn crate::context::ContextView> {
        Box::new(self.time.view())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn pcu_test() {}
}
