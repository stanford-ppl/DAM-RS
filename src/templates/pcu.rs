use crate::{
    channel::{ChannelElement, DequeueResult, EnqueueError, PeekResult, Receiver, Sender},
    context::Context,
    types::DAMType,
};

use super::ops::{ALUOp, PipelineRegister};
use dam_core::prelude::*;
use dam_macros::context;

pub enum Reg16Bit {
    U32(u32),
    F32(f32),
    I32(i32),
}

#[derive(Debug)]
pub struct PCUConfig<ElementType> {
    pub pipeline_depth: usize,
    pub num_registers: usize,
    pub counter: Option<ElementType>,
}

#[derive(Debug)]
pub struct PipelineStage<ET> {
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

trait IntoReceiver<OT> {
    fn peek_box(&self) -> PeekResult<OT>;
    fn peek_next_box(&self, manager: &TimeManager) -> DequeueResult<OT>;
    fn dequeue_box(&mut self, manager: &TimeManager) -> DequeueResult<OT>;
    fn attach_receiver_box(&self, receiver: &dyn Context);
}

impl<T: DAMType, OT: Clone> IntoReceiver<OT> for Receiver<T>
where
    T: Into<OT>,
{
    fn peek_box(&self) -> PeekResult<OT> {
        match self.peek() {
            PeekResult::Something(x) => PeekResult::Something(ChannelElement {
                time: x.time,
                data: x.data.into(),
            }),
            PeekResult::Nothing(x) => PeekResult::Nothing(x),
            PeekResult::Closed => PeekResult::Closed,
        }
    }

    fn peek_next_box(&self, manager: &TimeManager) -> DequeueResult<OT> {
        match self.peek_next(manager) {
            DequeueResult::Something(x) => DequeueResult::Something(ChannelElement {
                time: x.time,
                data: x.data.into(),
            }),
            DequeueResult::Closed => DequeueResult::Closed,
        }
    }

    fn dequeue_box(&mut self, manager: &TimeManager) -> DequeueResult<OT> {
        match self.dequeue(manager) {
            DequeueResult::Something(x) => DequeueResult::Something(ChannelElement {
                time: x.time,
                data: x.data.into(),
            }),
            DequeueResult::Closed => DequeueResult::Closed,
        }
    }

    fn attach_receiver_box(&self, receiver: &dyn Context) {
        self.attach_receiver(receiver)
    }
    // OT: enum
    // T: orignal datatype for the receiver
}

trait IntoSender<IT> {
    //fn wait_until_available_box(&mut self, manager: &TimeManager) -> Result<(), EnqueueError>;

    fn enqueue_box(
        &mut self,
        manager: &TimeManager,
        data: ChannelElement<IT>,
    ) -> Result<(), EnqueueError>;

    fn attach_sender_box(&self, sender: &dyn Context);
}

impl<T: DAMType, IT> IntoSender<IT> for Sender<T>
where
    IT: Into<T>,
{
    // IT: enum
    // T: sender datatype
    fn enqueue_box(
        &mut self,
        manager: &TimeManager,
        data: ChannelElement<IT>,
    ) -> Result<(), EnqueueError> {
        Sender::enqueue(
            self,
            manager,
            ChannelElement {
                time: data.time,
                data: data.data.into(),
            },
        )
    }

    fn attach_sender_box(&self, sender: &dyn Context) {
        self.attach_sender(sender)
    }
}

type InputChannelsType<ElementType> = Vec<Box<dyn IntoReceiver<ElementType> + Sync + Send>>;
type OutputChannelsType<ElementType> = Vec<Box<dyn IntoSender<ElementType> + Sync + Send>>;
type IngressOpType<ElementType> = fn(
    &InputChannelsType<ElementType>,
    &mut Vec<PipelineRegister<ElementType>>,
    &TimeManager,
    &Option<ElementType>,
) -> bool;

type EgressOpType<ElementType> =
    fn(&OutputChannelsType<ElementType>, &Vec<PipelineRegister<ElementType>>, Time, &TimeManager);

#[context]
pub struct PCU<ElementType: Clone> {
    configuration: PCUConfig<ElementType>,
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
        configuration: PCUConfig<ElementType>,
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
            configuration,
            registers,
            stages: Vec::with_capacity(pipe_depth),
            input_channels: vec![],
            output_channels: vec![],
            ingress_op,
            egress_op,
            context_info: Default::default(),
        }
    }

    pub const READ_ALL_INPUTS: IngressOpType<ElementType> = |ics, regs, time, cntr| {
        ics.iter().for_each(|recv| {
            recv.peek_next_box(time);
        });
        let reads: Vec<_> = ics.iter().map(|recv| (*recv).dequeue_box(time)).collect();

        for (ind, read) in reads.into_iter().enumerate() {
            match read {
                crate::channel::DequeueResult::Something(data) => regs[ind].data = data.data,
                crate::channel::DequeueResult::Closed => return false,
            }
        }

        true
    };

    pub const READ_ALL_INPUTS_COUNTER: IngressOpType<ElementType> = |ics, regs, time, cntr| {
        ics.iter().for_each(|recv| {
            recv.peek_next_box(time);
        });
        let reads: Vec<_> = ics.iter().map(|recv| recv.dequeue_box(time)).collect();

        match cntr {
            Some(x) => regs[0].data = x.clone(),
            None => return false,
        }

        for (ind, read) in reads.into_iter().enumerate() {
            match read {
                crate::channel::DequeueResult::Something(data) => regs[ind + 1].data = data.data,
                crate::channel::DequeueResult::Closed => return false,
            }
        }

        true
    };

    pub const WRITE_ALL_RESULTS: EgressOpType<ElementType> = |ocs, regs, out_time, manager| {
        ocs.iter().enumerate().for_each(|(ind, out_chan)| {
            (*out_chan)
                .enqueue_box(
                    manager,
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

    pub fn add_input_channel(&mut self, recv: impl IntoReceiver<ElementType> + Send + Sync) {
        recv.attach_receiver_box(self);
        self.input_channels.push(Box::new(recv));
    }

    pub fn add_output_channel(&mut self, send: impl IntoSender<ElementType> + Send + Sync) {
        send.attach_sender_box(self);
        self.output_channels.push(Box::new(send));
    }
}

impl<ElementType: DAMType> Context for PCU<ElementType>
// where
//     Box<dyn IntoReceiver<ElementType>>: Context,
//     Box<dyn IntoSender<ElementType>>: Context,
{
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            // Run the entire pipeline from front to back

            // We temporarily move the first registers out to avoid a mutable/immutable borrow conflict on self.
            let mut tmp_regs = std::mem::take(&mut self.registers[0]);
            if !(self.ingress_op)(
                &self.input_channels,
                &mut tmp_regs,
                &self.time,
                &self.configuration.counter,
            ) {
                return;
            }
            self.registers[0] = tmp_regs;

            for stage_index in 0..self.configuration.pipeline_depth {
                match self.stages.get(stage_index) {
                    Some(cur_stage) => {
                        let prev_regs = &self.registers[stage_index];
                        let next_regs = &self.registers[stage_index + 1];
                        let new_regs = cur_stage.run(prev_regs, next_regs);
                        self.registers[stage_index + 1] = new_regs;
                    }
                    None => self.registers[stage_index + 1] = self.registers[stage_index].clone(),
                }
            }

            let latency = u64::try_from(self.configuration.pipeline_depth).unwrap();

            // Need to provide a notion of time as to when this result is ready.
            (self.egress_op)(
                &self.output_channels,
                &self.registers[self.configuration.pipeline_depth],
                self.time.tick() + Time::new(latency),
                &self.time,
            );
            self.time.incr_cycles(1);
        }
    }

    fn ids(
        &self,
    ) -> std::collections::HashMap<VerboseIdentifier, std::collections::HashSet<VerboseIdentifier>>
    {
        std::collections::HashMap::from([(self.verbose(), std::collections::HashSet::new())])
    }

    fn edge_connections(&self) -> Option<crate::context::ExplicitConnections> {
        None
    }

    fn summarize(&self) -> crate::context::ContextSummary {
        crate::context::ContextSummary {
            id: self.verbose(),
            time: self.view(),
            children: vec![],
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::{simulation::*, templates::ops::*, utility_contexts::*};

    use super::PCU;

    #[test]
    fn pcu_test() {
        // two-stage PCU on scalars, with the third stage a no-op.
        let mut parent = ProgramBuilder::default();

        const CHAN_SIZE: usize = 8;
        let ingress_op = PCU::<u16>::READ_ALL_INPUTS;
        let egress_op = PCU::<u16>::WRITE_ALL_RESULTS;

        let mut pcu = PCU::<u16>::new(
            super::PCUConfig::<u16> {
                pipeline_depth: 3,
                num_registers: 3,
                counter: None,
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

        let (arg1_send, arg1_recv) = parent.bounded(CHAN_SIZE);
        let (arg2_send, arg2_recv) = parent.bounded(CHAN_SIZE);
        let (arg3_send, arg3_recv) = parent.bounded(CHAN_SIZE);
        let (pcu_out_send, pcu_out_recv) = parent.bounded(CHAN_SIZE);

        pcu.add_input_channel(arg1_recv);
        pcu.add_input_channel(arg2_recv);
        pcu.add_input_channel(arg3_recv);
        pcu.add_output_channel(pcu_out_send);

        let gen1 = GeneratorContext::new(|| (0u16..32), arg1_send);
        let gen2 = GeneratorContext::new(|| (32u16..64), arg2_send);
        let gen3 = GeneratorContext::new(|| (64u16..96), arg3_send);
        let checker = CheckerContext::new(
            || {
                (0u16..32)
                    .zip(32u16..64)
                    .zip(64u16..96)
                    .map(|((a, b), c)| (a * b) + c)
            },
            pcu_out_recv,
        );

        parent.add_child(gen1);
        parent.add_child(gen2);
        parent.add_child(gen3);
        parent.add_child(pcu);
        parent.add_child(checker);
        parent
            .initialize(InitializationOptions {
                run_flavor_inference: true,
            })
            .unwrap()
            .run(RunMode::Simple);
    }
}
