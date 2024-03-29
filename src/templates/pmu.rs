//! An implementation of the Pattern Memory Unit from the Plasticine paper.
//! Unlike the original PMU, this does not support processing within the pipeline, and so all values must be pre-computed before being sent here.

use std::{
    cmp::max,
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    channel::{
        utils::{EventTime, Peekable},
        ChannelID,
    },
    context::{self, Context, ContextSummary, ExplicitConnections, ProxyContext},
    datastructures::{Identifiable, Identifier, Time, VerboseIdentifier},
    logging::{copy_log, initialize_log},
    types::{Cleanable, DAMType, IndexLike},
    view::{ContextView, ParentView, TimeView, TimeViewable},
};

use crate::context_tools::*;
use dam_macros::context_internal;

use super::datastore::{self, Behavior, Datastore};

/// A PMU consists of a reader and a writer, which may operate simultaneously.
/// Each reader and writer can process one request per cycle, but may both be active (to support use-cases such as FIFOs).
pub struct PMU<T: DAMType, IT: IndexLike, AT: DAMType> {
    reader: ProxyContext<ReadPipeline<T, IT>>,
    writer: ProxyContext<WritePipeline<T, IT, AT>>,
    identifier: Identifier,
}

impl<T: DAMType, IT: IndexLike, AT: DAMType> Context for PMU<T, IT, AT> {
    fn init(&mut self) {
        self.reader.init();
        self.writer.init();
    }

    fn run(&mut self) {
        let read_log = copy_log();
        let write_log = copy_log();
        crate::shim::scope(|s| {
            crate::shim::spawn!(s, || {
                if let Some(mut logger) = read_log {
                    logger.id = self.reader.id();
                    initialize_log(logger);
                }
                self.reader.run();
                self.reader.cleanup();
            })
            .unwrap();
            crate::shim::spawn!(s, || {
                if let Some(mut logger) = write_log {
                    logger.id = self.writer.id();
                    initialize_log(logger);
                }
                self.writer.run();
                self.writer.cleanup();
            })
            .unwrap();
        });
    }

    fn ids(&self) -> HashMap<VerboseIdentifier, HashSet<VerboseIdentifier>> {
        let mut base: HashMap<_, _> = [(
            self.verbose(),
            [self.reader.verbose(), self.writer.verbose()].into(),
        )]
        .into();
        base.extend(self.reader.ids());
        base.extend(self.writer.ids());
        base
    }

    fn edge_connections(&self) -> Option<ExplicitConnections> {
        let mut result =
            HashMap::<Identifier, Vec<(HashSet<ChannelID>, HashSet<ChannelID>)>>::new();
        for bundle in &self.reader.readers {
            let srcs = [bundle.addr.id()];
            let dsts = [bundle.resp.id()];
            result.insert(self.reader.id(), vec![(srcs.into(), dsts.into())]);
        }
        for bundle in &self.writer.writers {
            let srcs = [bundle.addr.id(), bundle.data.id()];
            let dsts = [bundle.ack.id()];
            result.insert(self.writer.id(), vec![(srcs.into(), dsts.into())]);
        }
        Some(result)
    }

    fn summarize(&self) -> context::ContextSummary {
        ContextSummary {
            id: self.verbose(),
            time: self.view(),
            children: vec![self.reader.summarize(), self.writer.summarize()],
        }
    }
}

impl<T: DAMType, IT: IndexLike, AT: DAMType> Identifiable for PMU<T, IT, AT> {
    fn id(&self) -> Identifier {
        self.identifier
    }

    fn name(&self) -> String {
        "PMU".to_string()
    }
}

impl<T: DAMType, IT: IndexLike, AT: DAMType> TimeViewable for PMU<T, IT, AT> {
    fn view(&self) -> TimeView {
        (ParentView {
            child_views: vec![self.writer.view(), self.reader.view()],
        })
        .into()
    }
}

impl<T: DAMType, IT: IndexLike, AT: DAMType> PMU<T, IT, AT> {
    /// Constructs a new PMU with a given capacity and datastore [Behavior]
    pub fn new(capacity: usize, behavior: Behavior) -> PMU<T, IT, AT> {
        // This could probably somehow be embedded into the PMU instead of being an Arc.
        let datastore = Arc::new(Datastore::new(capacity, behavior));
        let mut pmu = PMU {
            reader: ReadPipeline {
                readers: Default::default(),
                datastore: datastore.clone(),
                writer_view: None,
                context_info: Default::default(),
            }
            .into(),
            writer: WritePipeline {
                writers: Default::default(),
                datastore,
                context_info: Default::default(),
            }
            .into(),
            identifier: Identifier::new(),
        };
        pmu.reader.writer_view = Some(pmu.writer.view());
        dbg!(pmu.id());
        dbg!(pmu.reader.id());
        dbg!(pmu.writer.id());

        pmu
    }

    /// Registers a new reader
    pub fn add_reader(&mut self, reader: PMUReadBundle<T, IT>) {
        self.reader.add_reader(reader);
    }

    /// Registers a new writer
    pub fn add_writer(&mut self, writer: PMUWriteBundle<T, IT, AT>) {
        self.writer.add_writer(writer);
    }
}

/// Represents a reader, which receives addresses and writes out the data at the address.
pub struct PMUReadBundle<T: Clone, IT: Clone> {
    /// An input stream consisting of addresses to read from.
    pub addr: Receiver<IT>,
    /// An output stream which will hold addresses.
    pub resp: Sender<T>,
}

/// Represents a writer, which sends in addresses and data, receiving an acknowledgement after the write is processed.
pub struct PMUWriteBundle<T: Clone, IT: Clone, AT: Clone> {
    /// The write address
    pub addr: Receiver<IT>,
    /// The data to be written
    pub data: Receiver<T>,
    /// The acknowledgement channel, which is written to each time a write is successfully processed.
    pub ack: Sender<AT>,
}

#[context_internal]
struct ReadPipeline<T, IT>
where
    T: DAMType,
    IT: IndexLike,
{
    readers: Vec<PMUReadBundle<T, IT>>,
    datastore: Arc<datastore::Datastore<T>>,
    writer_view: Option<TimeView>,
}

impl<T: DAMType, IT: IndexLike> ReadPipeline<T, IT>
where
    ReadPipeline<T, IT>: context::Context,
{
    pub fn add_reader(&mut self, reader: PMUReadBundle<T, IT>) {
        let rd = reader;
        rd.addr.attach_receiver(self);
        rd.resp.attach_sender(self);
        self.readers.push(rd);
    }

    fn await_writer(&mut self) -> Time {
        self.writer_view
            .as_ref()
            .unwrap()
            .wait_until(self.time.tick())
    }
}

impl<T: DAMType, IT: IndexLike> Context for ReadPipeline<T, IT> {
    fn init(&mut self) {}

    fn run(&mut self) {
        loop {
            let next_events: Vec<EventTime> = self
                .readers
                .iter_mut()
                .map(|reader| reader.addr.next_event())
                .collect();
            let next_event = next_events
                .iter()
                .enumerate()
                .min_by_key(|(_, index)| *index);
            let (event_ind, event_time) = match next_event {
                None | Some((_, EventTime::Closed)) => {
                    // No more events!
                    return;
                }
                Some((ind, time)) => (ind, time),
            };
            match event_time {
                EventTime::Ready(time) => self.time.advance(*time),
                EventTime::Nothing(time) => {
                    self.time.advance(*time + 1);
                    continue;
                }
                EventTime::Closed => unreachable!(),
            }
            // Wait for the writer to catch up. At this point in time, self.tick should be the same as the ready time
            // so the subsequent dequeue shouldn't actually change the tick time.
            let _ = self.await_writer();
            // At this point, we have advanced to the time of the ready!
            let deq_reader = self.readers.get(event_ind).unwrap();
            let elem = deq_reader.addr.dequeue(&self.time).unwrap();

            let addr: usize = elem.data.to_usize();
            let cur_time = self.time.tick();
            let rv = self.datastore.read(addr, cur_time);
            deq_reader
                .resp
                .enqueue(&self.time, ChannelElement::new(cur_time, rv))
                .unwrap();
            self.time.incr_cycles(1);
        }
    }
}

#[context_internal]
struct WritePipeline<T: DAMType, IT: DAMType, AT: DAMType> {
    writers: Vec<PMUWriteBundle<T, IT, AT>>,
    datastore: Arc<Datastore<T>>,
}

impl<T: DAMType, IT: IndexLike, AT: DAMType> WritePipeline<T, IT, AT>
where
    WritePipeline<T, IT, AT>: context::Context,
{
    pub fn add_writer(&mut self, writer: PMUWriteBundle<T, IT, AT>) {
        let wr = writer;
        wr.addr.attach_receiver(self);
        wr.data.attach_receiver(self);
        wr.ack.attach_sender(self);
        self.writers.push(wr);
    }
}

impl<T: DAMType, IT: IndexLike, AT: DAMType> Context for WritePipeline<T, IT, AT> {
    fn init(&mut self) {} // Do nothing for init

    fn run(&mut self) {
        loop {
            let next_events: Vec<EventTime> = self
                .writers
                .iter_mut()
                .map(|writer: &mut PMUWriteBundle<T, IT, AT>| {
                    max(writer.addr.next_event(), writer.data.next_event())
                })
                .collect();
            let next_event = next_events
                .iter()
                .enumerate()
                .min_by_key(|(_, index)| *index);
            let (event_ind, event_time) = match next_event {
                None | Some((_, EventTime::Closed)) => {
                    // No more events!
                    return;
                }
                Some((ind, time)) => (ind, time),
            };

            match event_time {
                EventTime::Ready(time) => self.time.advance(*time),
                EventTime::Nothing(time) => {
                    self.time.advance(*time + 1);
                    continue;
                }
                EventTime::Closed => unreachable!(),
            }

            let deq_writer = self.writers.get(event_ind).unwrap();
            let addr_elem = deq_writer.addr.dequeue(&self.time).unwrap();
            let data_elem = deq_writer.data.dequeue(&self.time).unwrap();

            let addr: usize = addr_elem.data.to_usize();
            let cur_time = self.time.tick();
            self.datastore.write(addr, data_elem.data, self.time.tick());
            deq_writer
                .ack
                .enqueue(&self.time, ChannelElement::new(cur_time, AT::default()))
                .unwrap();

            self.time.incr_cycles(1);
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        channel::ChannelElement,
        simulation::*,
        templates::{
            datastore::Behavior,
            pmu::{PMUReadBundle, PMUWriteBundle},
        },
        utility_contexts::*,
    };

    use super::PMU;

    #[test]
    fn simple_pmu_test() {
        const TEST_SIZE: usize = 64;
        let mut parent = ProgramBuilder::default();
        const CHAN_SIZE: usize = TEST_SIZE;

        let mut pmu = PMU::<u16, u16, bool>::new(
            TEST_SIZE,
            Behavior {
                mod_address: false,
                use_default_value: false,
            },
        );

        let (wr_addr_send, wr_addr_recv) = parent.bounded(CHAN_SIZE);
        let (wr_data_send, wr_data_recv) = parent.bounded(CHAN_SIZE);
        let (wr_ack_send, wr_ack_recv) = parent.bounded(CHAN_SIZE);

        let write_addr_gen = GeneratorContext::new(
            move || (0..TEST_SIZE).map(|x| u16::try_from(x).unwrap()),
            wr_addr_send,
        );
        parent.add_child(write_addr_gen);

        let wr_data_gen = GeneratorContext::new(
            move || (0..TEST_SIZE).map(|x| u16::try_from(x).unwrap()),
            wr_data_send,
        );

        parent.add_child(wr_data_gen);

        pmu.add_writer(PMUWriteBundle {
            addr: wr_addr_recv,
            data: wr_data_recv,
            ack: wr_ack_send,
        });

        let (rd_addr_send, rd_addr_recv) = parent.bounded(CHAN_SIZE);
        let (rd_data_send, rd_data_recv) = parent.bounded(CHAN_SIZE);
        pmu.add_reader(PMUReadBundle {
            addr: rd_addr_recv,
            resp: rd_data_send,
        });

        let mut rd_addr_gen = FunctionContext::new();
        wr_ack_recv.attach_receiver(&rd_addr_gen);
        rd_addr_send.attach_sender(&rd_addr_gen);
        rd_addr_gen.set_run(move |time| {
            for ind in 0..TEST_SIZE {
                wr_ack_recv.dequeue(time).unwrap();
                let send_time = time.tick();
                rd_addr_send
                    .enqueue(
                        time,
                        ChannelElement {
                            time: send_time,
                            data: u16::try_from(ind).unwrap(),
                        },
                    )
                    .unwrap();
            }
        });

        parent.add_child(rd_addr_gen);

        let checker = CheckerContext::new(
            || (0..TEST_SIZE).map(|x| u16::try_from(x).unwrap()),
            rd_data_recv,
        );
        parent.add_child(checker);

        parent.add_child(pmu);
        let initialized = parent.initialize(InitializationOptions::default()).unwrap();
        #[cfg(feature = "dot")]
        println!("{}", initialized.to_dot_string());

        let run_options = RunOptionsBuilder::default().log_filter(LogFilterKind::Blanket(
            crate::logging::LogFilter::Some(["TimeEvent".to_string()].into()),
        ));
        cfg_if::cfg_if! {
            if #[cfg(feature = "test-log-mongo")] {
                let run_options = run_options.logging(LoggingOptions::Mongo(
                    MongoOptionsBuilder::default()
                        .db("pmu_log".to_string())
                        .uri("mongodb://127.0.0.1:27017".to_string())
                        .build()
                        .unwrap(),
                ));
            }
        }

        let summary = initialized.run(run_options.build().unwrap());
        dbg!(summary.elapsed_cycles());
        #[cfg(feature = "dot")]
        {
            println!("{}", summary.to_dot_string());
        }
    }
}
