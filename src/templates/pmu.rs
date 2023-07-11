use std::{cmp::max, sync::Arc};

use crate::{
    channel::{
        utils::{dequeue, enqueue, EventTime, Peekable},
        ChannelElement, Receiver, Sender,
    },
    context::{self, Context},
    types::{DAMType, IndexLike},
};

use dam_core::{
    identifier::{Identifiable, Identifier},
    time::Time,
    ContextView, ParentView, TimeManaged, TimeView, TimeViewable,
};
use dam_macros::{cleanup, identifiable, time_managed};

use super::datastore::{self, Behavior, Datastore};

#[identifiable]
pub struct PMU<T: DAMType, IT: IndexLike, AT: DAMType> {
    reader: ReadPipeline<T, IT>,
    writer: WritePipeline<T, IT, AT>,
}

impl<T: DAMType, IT: IndexLike, AT: DAMType> Context for PMU<T, IT, AT> {
    fn init(&mut self) {
        self.reader.init();
        self.writer.init();
    }

    fn run(&mut self) {
        std::thread::scope(|s| {
            s.spawn(|| {
                self.reader.run();
                self.reader.cleanup();
            });
            s.spawn(|| {
                self.writer.run();
                self.writer.cleanup();
            });
        });
    }

    fn cleanup(&mut self) {} // No-op
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
    pub fn new(capacity: usize, behavior: Behavior) -> PMU<T, IT, AT> {
        // This could probably somehow be embedded into the PMU instead of being an Arc.
        let datastore = Arc::new(Datastore::new(capacity, behavior));
        let mut pmu = PMU {
            reader: ReadPipeline {
                readers: Default::default(),
                datastore: datastore.clone(),
                writer_view: None,
                identifier: Identifier::new(),
                time: Default::default(),
            },
            writer: WritePipeline {
                writers: Default::default(),
                datastore,
                identifier: Identifier::new(),
                time: Default::default(),
            },
            identifier: Identifier::new(),
        };
        pmu.reader.writer_view = Some(pmu.writer.view());
        let mut handle = dam_core::log_graph::register_handle(pmu.id());
        handle.add_child(pmu.reader.id());
        handle.add_child(pmu.writer.id());
        pmu
    }

    pub fn add_reader(&mut self, reader: PMUReadBundle<T, IT>) {
        self.reader.add_reader(reader);
    }

    pub fn add_writer(&mut self, writer: PMUWriteBundle<T, IT, AT>) {
        self.writer.add_writer(writer);
    }
}

pub struct PMUReadBundle<T, IT> {
    pub addr: Receiver<IT>,
    pub resp: Sender<T>,
}

pub struct PMUWriteBundle<T, IT, AT> {
    pub addr: Receiver<IT>,
    pub data: Receiver<T>,
    pub ack: Sender<AT>,
}

#[identifiable]
#[time_managed]
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
            .as_mut()
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
                EventTime::Ready(time) => self.time_manager_mut().advance(*time),
                EventTime::Nothing(time) => {
                    self.time_manager_mut().advance(*time + 1);
                    continue;
                }
                EventTime::Closed => unreachable!(),
            }
            // Wait for the writer to catch up. At this point in time, self.tick should be the same as the ready time
            // so the subsequent dequeue shouldn't actually change the tick time.
            let _ = self.await_writer();
            // At this point, we have advanced to the time of the ready!
            let deq_reader = self.readers.get_mut(event_ind).unwrap();
            let elem = dequeue(&mut self.time, &mut deq_reader.addr).unwrap();

            let addr: usize = elem.data.to_usize();
            let cur_time = self.time.tick();
            let rv = self.datastore.read(addr, cur_time);
            enqueue(
                &mut self.time,
                &mut deq_reader.resp,
                ChannelElement::new(cur_time, rv),
            )
            .unwrap();
            self.time_manager_mut().incr_cycles(1);
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.readers.clear();
    }
}

#[identifiable]
#[time_managed]
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

            let deq_writer = self.writers.get_mut(event_ind).unwrap();
            let addr_elem = dequeue(&mut self.time, &mut deq_writer.addr).unwrap();
            let data_elem = dequeue(&mut self.time, &mut deq_writer.data).unwrap();

            let addr: usize = addr_elem.data.to_usize();
            let cur_time = self.time.tick();
            self.datastore.write(addr, data_elem.data, self.time.tick());
            enqueue(
                &mut self.time,
                &mut deq_writer.ack,
                ChannelElement::new(cur_time, AT::default()),
            )
            .unwrap();

            self.time.incr_cycles(1);
        }
    }

    #[cleanup(time_managed)]
    fn cleanup(&mut self) {
        self.writers.clear();
    }
}

#[cfg(test)]
mod tests {

    use dam_core::{log_graph::get_graph, ContextView, TimeViewable};

    use crate::{
        channel::{
            unbounded,
            utils::{dequeue, enqueue},
            ChannelElement,
        },
        context::{
            checker_context::CheckerContext, function_context::FunctionContext,
            generator_context::GeneratorContext, parent::BasicParentContext, Context,
            ParentContext,
        },
        templates::{
            datastore::Behavior,
            pmu::{PMUReadBundle, PMUWriteBundle},
        },
    };

    use super::PMU;

    #[test]
    fn simple_pmu_test() {
        const TEST_SIZE: usize = 1024 * 64;
        let mut parent = BasicParentContext::default();

        let mut pmu = PMU::<u16, u16, bool>::new(
            TEST_SIZE,
            Behavior {
                mod_address: false,
                use_default_value: false,
            },
        );

        let (wr_addr_send, wr_addr_recv) = unbounded::<u16>();
        let (wr_data_send, wr_data_recv) = unbounded::<u16>();
        let (wr_ack_send, mut wr_ack_recv) = unbounded::<bool>();

        let mut write_addr_gen = GeneratorContext::new(
            move || (0..TEST_SIZE).map(|x| u16::try_from(x).unwrap()),
            wr_addr_send,
        );
        parent.add_child(&mut write_addr_gen);

        let mut wr_data_gen = GeneratorContext::new(
            move || (0..TEST_SIZE).map(|x| u16::try_from(x).unwrap()),
            wr_data_send,
        );

        parent.add_child(&mut wr_data_gen);

        pmu.add_writer(PMUWriteBundle {
            addr: wr_addr_recv,
            data: wr_data_recv,
            ack: wr_ack_send,
        });

        let (mut rd_addr_send, rd_addr_recv) = unbounded::<u16>();
        let (rd_data_send, rd_data_recv) = unbounded::<u16>();
        pmu.add_reader(PMUReadBundle {
            addr: rd_addr_recv,
            resp: rd_data_send,
        });

        let mut rd_addr_gen = FunctionContext::new();
        wr_ack_recv.attach_receiver(&rd_addr_gen);
        rd_addr_send.attach_sender(&rd_addr_gen);
        rd_addr_gen.set_run(move |time| {
            for ind in 0..TEST_SIZE {
                dequeue(time, &mut wr_ack_recv).unwrap();
                let send_time = time.tick();
                enqueue(
                    time,
                    &mut rd_addr_send,
                    ChannelElement {
                        time: send_time,
                        data: u16::try_from(ind).unwrap(),
                    },
                )
                .unwrap();
            }
        });

        parent.add_child(&mut rd_addr_gen);

        let mut checker = CheckerContext::new(
            || (0..TEST_SIZE).map(|x| u16::try_from(x).unwrap()),
            rd_data_recv,
        );
        parent.add_child(&mut checker);

        parent.add_child(&mut pmu);
        parent.init();
        parent.run();
        parent.cleanup();
        let finish_time = pmu.view().tick_lower_bound();
        dbg!(finish_time);
        assert!(finish_time.is_infinite());
        assert_eq!(finish_time.time(), u64::try_from(TEST_SIZE).unwrap() + 1);

        get_graph().dump();
    }
}
