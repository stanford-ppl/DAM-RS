use std::sync::{Arc, Mutex, RwLock};

use crate::{
    channel::{Receiver, Sender},
    context::{view::TimeManager, Context},
    time::Time,
    types::DAMType,
};

#[derive(Default, Debug)]
pub struct PipelineRegister<T> {
    underlying: T,
}

#[derive(Debug)]
pub struct PCUConfig {
    pipeline_depth: usize,
    num_registers: usize,
    initiation_interval: Time,
}

#[derive(Debug)]
pub struct PCUOp<T, DatastoreType> {
    func: fn(&[T], &mut [T], &mut DatastoreType),
    name: String,
}

#[derive(Debug)]
pub struct PCUStage<T, DatastoreType> {
    op: Arc<PCUOp<T, DatastoreType>>,
    input_register_ids: Vec<usize>,
    output_register_ids: Vec<usize>,
}

pub struct PCU<ElementType, DatastoreType> {
    time: TimeManager,
    configuration: PCUConfig,
    datastore: DatastoreType,
    registers: Vec<Vec<PipelineRegister<ElementType>>>,

    // The operation to run, and the pipeline registers to forward
    stages: Vec<(PCUStage<ElementType, DatastoreType>, Vec<(usize, usize)>)>,

    input_channels: Vec<Arc<Mutex<Receiver<ElementType>>>>,
    output_channels: Vec<Arc<Mutex<Sender<ElementType>>>>,

    ingress_op: Arc<
        Mutex<
            dyn Fn(
                    &mut Vec<Arc<Mutex<Receiver<ElementType>>>>,
                    &mut DatastoreType,
                    &mut Vec<PipelineRegister<ElementType>>,
                ) + Sync
                + Send,
        >,
    >,

    egress_op: Arc<
        Mutex<
            dyn Fn(
                    &mut Vec<Arc<Mutex<Sender<ElementType>>>>,
                    &mut DatastoreType,
                    &mut Vec<PipelineRegister<ElementType>>,
                ) + Sync
                + Send,
        >,
    >,
}

impl<ElementType: DAMType, DatastoreType> PCU<ElementType, DatastoreType> {}

impl<ElementType: DAMType, DatastoreType: Sync + Send> Context for PCU<ElementType, DatastoreType> {
    fn init(&mut self) {
        self.registers
            .resize_with(self.configuration.pipeline_depth, || {
                let mut registers = Vec::<PipelineRegister<ElementType>>::new();
                registers.resize_with(self.configuration.num_registers, Default::default);
                registers
            });
    }

    fn run(&mut self) {
        // Run the pipeline back-to-front to handle stalling
    }

    fn cleanup(&mut self) {
        todo!()
    }

    fn view(&self) -> Box<dyn crate::context::ContextView> {
        Box::new(self.time.view())
    }
}
