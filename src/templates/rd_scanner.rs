use std::sync::{Arc, Mutex};

pub mod primitive;

use crate::{
    channel::{Receiver, Sender},
    context::{view::TimeManager, Context},
    time::Time,
    types::DAMType,
};

pub struct rd_scan_data {
    curr_ref: Stream,
    curr_crd: Stream,
    in_ref: Vec<Stream>,
    end_fiber: bool,
    emit_tkn: bool,
    meta_dim: i32,
    start_addr: i32,
    end_addr: i32,
    begin: bool,
}

pub struct CompressCrdRdScan {
    data: rd_scan_data,
}

impl<ElementType: DAMType> Context for CrdRdScan<ElementType> {
    fn init(&mut self) {
        data.
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
