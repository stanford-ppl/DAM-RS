use std::{cmp::max, sync::Arc};

use crate::{
    channel::{
        utils::{dequeue, enqueue, EventTime, Peekable},
        ChannelElement, Receiver, Sender,
    },
    context::{view::TimeManager, Context, ContextView},
    time::Time,
    types::DAMType,
};

use super::datastore::{self, Behavior, Datastore};

pub struct PMU<T: DAMType, IT: DAMType, AT: DAMType>
where
    usize: From<IT>,
{
    time: TimeManager,
    reader: ReadPipeline<T, IT>,
    writer: WritePipeline<T, IT, AT>,
}

impl<T: DAMType, IT: DAMType, AT: DAMType> Context<'_> for PMU<T, IT, AT>
where
    usize: From<IT>,
{
    fn init(&mut self) {
        self.reader.init();
        self.writer.init();
    }

    fn run(&mut self) {
        rayon::in_place_scope(|s| {
            s.spawn(|_| {
                self.reader.run();
                self.reader.cleanup();
            });
            s.spawn(|_| {
                self.writer.run();
                self.writer.cleanup();
            });
        });
    }

    fn cleanup(&mut self) {} // No-op

    fn view(&self) -> Box<dyn crate::context::ContextView> {
        Box::new(self.time.view())
    }
}

impl<T: DAMType, IT: DAMType, AT: DAMType> PMU<T, IT, AT>
where
    usize: From<IT>,
{
    pub fn new(capacity: usize, behavior: Behavior) -> PMU<T, IT, AT> {
        // This could probably somehow be embedded into the PMU instead of being an Arc.
        let datastore = Arc::new(Datastore::new(capacity, behavior));
        let mut pmu = PMU {
            time: TimeManager::new(),
            reader: ReadPipeline {
                readers: Default::default(),
                time: Default::default(),
                datastore: datastore.clone(),
                writer_view: None,
            },
            writer: WritePipeline {
                writers: Default::default(),
                time: Default::default(),
                datastore,
            },
        };
        pmu.reader.writer_view = Some(pmu.writer.view());
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

struct ReadPipeline<T: DAMType, IT: DAMType> {
    time: TimeManager,
    readers: Vec<PMUReadBundle<T, IT>>,
    datastore: Arc<datastore::Datastore<T>>,
    writer_view: Option<Box<dyn ContextView>>,
}

impl<T: DAMType, IT: DAMType> ReadPipeline<T, IT> {
    pub fn add_reader(&mut self, reader: PMUReadBundle<T, IT>) {
        self.readers.push(reader);
    }

    fn await_writer(&mut self) -> crossbeam::channel::Receiver<Time> {
        self.writer_view
            .as_mut()
            .unwrap()
            .signal_when(self.time.tick())
    }
}

impl<T: DAMType, IT: DAMType> Context<'_> for ReadPipeline<T, IT>
where
    usize: From<IT>,
{
    fn init(&mut self) {} // Do nothing for init

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

            println!("Next Read Event: {event_time:?}");
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
            let _ = self.await_writer().recv().unwrap();
            // At this point, we have advanced to the time of the ready!
            let deq_reader = self.readers.get_mut(event_ind).unwrap();
            let elem = dequeue(&mut self.time, &mut deq_reader.addr).unwrap();

            let addr: usize = elem.data.into();
            let cur_time = self.time.tick();
            let rv = self.datastore.read(addr, cur_time);
            enqueue(
                &mut self.time,
                &mut deq_reader.resp,
                ChannelElement::new(cur_time, rv),
            )
            .unwrap();
            self.time.incr_cycles(1);
        }
    }

    fn cleanup(&mut self) {
        self.readers.clear();
        println!("Reader finished at {:?}", self.time.tick());
        self.time.cleanup();
    }

    fn view(&self) -> Box<dyn crate::context::ContextView> {
        Box::new(self.time.view())
    }
}

struct WritePipeline<T: DAMType, IT: DAMType, AT: DAMType> {
    time: TimeManager,
    writers: Vec<PMUWriteBundle<T, IT, AT>>,
    datastore: Arc<Datastore<T>>,
}

impl<T: DAMType, IT: DAMType, AT: DAMType> WritePipeline<T, IT, AT> {
    pub fn add_writer(&mut self, writer: PMUWriteBundle<T, IT, AT>) {
        self.writers.push(writer);
    }
}

impl<T: DAMType, IT: DAMType, AT: DAMType> Context<'_> for WritePipeline<T, IT, AT>
where
    usize: From<IT>,
{
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

            println!("Next Write Event: {event_time:?}");

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

            let addr: usize = addr_elem.data.into();
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

    fn cleanup(&mut self) {
        self.writers.clear();
        println!("Writer finished at {:?}", self.time.tick());
        self.time.cleanup();
    }

    fn view(&self) -> Box<dyn crate::context::ContextView> {
        Box::new(self.time.view())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::{
        channel::{
            utils::{dequeue, enqueue},
            Bounded, ChannelElement,
        },
        context::{
            function_context::FunctionContext, parent::BasicParentContext, Context, ParentContext,
        },
        templates::datastore::Behavior,
    };

    use super::PMU;

    #[test]
    fn simple_pmu_test() {
        const TEST_SIZE: usize = 32;
        let mut pmu = PMU::<i32, u16, bool>::new(
            TEST_SIZE,
            Behavior {
                mod_address: false,
                use_default_value: false,
            },
        );

        let mut write_issue = FunctionContext::default();
        let mut read_issue = FunctionContext::default();
        let mut checker = FunctionContext::default();
        let (write_ack_send, write_ack_recv) = Bounded::<bool>::make(8, &pmu.writer, &read_issue);
        let (write_addr_send, write_addr_recv) = Bounded::<u16>::make(8, &write_issue, &pmu.writer);
        let (write_data_send, write_data_recv) = Bounded::<i32>::make(8, &write_issue, &pmu.writer);
        let (read_addr_send, read_addr_recv) = Bounded::<u16>::make(8, &read_issue, &pmu.reader);
        let (read_data_send, read_data_recv) = Bounded::<i32>::make(8, &pmu.reader, &checker);

        let wr_addr_send = Arc::new(Mutex::new(write_addr_send));
        let wr_data_send = Arc::new(Mutex::new(write_data_send));
        // Set up the write issuer
        {
            let was = wr_addr_send.clone();
            let wds = wr_data_send.clone();
            write_issue.set_run(Arc::new(move |ctx| {
                let mut addr = was.lock().unwrap();
                let mut data = wds.lock().unwrap();
                for i in 0..TEST_SIZE {
                    let tick = ctx.time.tick();
                    println!("Write Issue iteration {i} at time {tick:?}");
                    enqueue(
                        &mut ctx.time,
                        &mut addr,
                        ChannelElement::new(tick, u16::try_from(i).unwrap()),
                    )
                    .unwrap();

                    enqueue(
                        &mut ctx.time,
                        &mut data,
                        ChannelElement::new(tick, i32::try_from(i).unwrap()),
                    )
                    .unwrap();
                    ctx.time.incr_cycles(1);
                }
            }));
        }
        {
            let was = wr_addr_send;
            let wds = wr_data_send;
            write_issue.set_cleanup(Arc::new(move |ctx| {
                was.lock().unwrap().close();
                wds.lock().unwrap().close();
                ctx.time.cleanup()
            }));
        }

        let wack = Arc::new(Mutex::new(write_ack_recv));
        let raddr_send = Arc::new(Mutex::new(read_addr_send));
        // Set up the read issuer
        {
            let wackr = wack.clone();
            let raddr = raddr_send.clone();
            read_issue.set_run(Arc::new(move |ctx| {
                let mut wack = wackr.lock().unwrap();
                let mut raddr = raddr.lock().unwrap();
                for i in 0..TEST_SIZE {
                    let _ = dequeue(&mut ctx.time, &mut wack).unwrap();
                    let tick = ctx.time.tick();
                    println!("Read Issue iteration {i} at time {tick:?}");
                    enqueue(
                        &mut ctx.time,
                        &mut raddr,
                        ChannelElement::new(tick, i.try_into().unwrap()),
                    )
                    .unwrap();
                    ctx.time.incr_cycles(1);
                }
            }));
        }

        {
            let wackr = wack;
            let raddr = raddr_send;
            read_issue.set_cleanup(Arc::new(move |ctx| {
                wackr.lock().unwrap().close();
                raddr.lock().unwrap().close();
                ctx.time.cleanup()
            }));
        }

        let rdata_recv = Arc::new(Mutex::new(read_data_recv));

        {
            let rdatar = rdata_recv;
            checker.set_run(Arc::new(move |ctx| {
                let mut rdata = rdatar.lock().unwrap();
                for i in 0..TEST_SIZE {
                    let d = dequeue(&mut ctx.time, &mut rdata).unwrap();
                    println!("Checker iteration {i}");
                    let gold: i32 = i.try_into().unwrap();
                    assert_eq!(gold, d.data);
                    ctx.time.incr_cycles(1);
                }
            }))
        }

        pmu.add_writer(super::PMUWriteBundle {
            addr: write_addr_recv,
            data: write_data_recv,
            ack: write_ack_send,
        });

        pmu.add_reader(super::PMUReadBundle {
            addr: read_addr_recv,
            resp: read_data_send,
        });

        let mut parent = BasicParentContext::default();
        println!("Finished setup!");
        parent.add_child(&mut pmu);
        parent.add_child(&mut write_issue);
        parent.add_child(&mut read_issue);
        parent.add_child(&mut checker);
        parent.init();
        parent.run();
        parent.cleanup();
    }
}
