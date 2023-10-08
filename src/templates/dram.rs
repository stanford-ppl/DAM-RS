use std::collections::VecDeque;

use crate::{
    channel::{
        utils::{EventTime, Peekable},
        ChannelElement, Receiver, Sender,
    },
    context::Context,
    datastructures::Time,
    types::{DAMType, IndexLike},
};

use dam_macros::context_internal;

use super::datastore::{Behavior, Datastore};

#[derive(Clone, Copy, Debug)]
pub struct DRAMConfig {
    num_simultaneous_requests: usize,
    bandwidth_in_bits: usize,
    latency: Time,
    capacity: usize,
}

pub struct DRAMReadBundle<IT: Clone, DT: Clone> {
    addr: Receiver<IT>,
    req_size: Receiver<IT>,
    data: Sender<DT>,
}

impl<IT: DAMType, DT: Clone> Peekable for DRAMReadBundle<IT, DT> {
    fn next_event(&mut self) -> crate::channel::utils::EventTime {
        [self.addr.next_event(), self.req_size.next_event()]
            .into_iter()
            .max()
            .unwrap()
    }
}

pub struct DRAMWriteBundle<IT: DAMType, DT: DAMType, AT: Clone> {
    addr: Receiver<IT>,
    request_size: Receiver<IT>,
    data: Receiver<DT>,
    ack: Sender<AT>,
}

impl<IT: DAMType, DT: DAMType, AT: Clone> Peekable for DRAMWriteBundle<IT, DT, AT> {
    fn next_event(&mut self) -> crate::channel::utils::EventTime {
        [
            self.addr.next_event(),
            self.data.next_event(),
            self.request_size.next_event(),
        ]
        .into_iter()
        .max()
        .unwrap()
    }
}

enum AccessBundle<IT: DAMType, DT: DAMType, AT: Clone> {
    Write(DRAMWriteBundle<IT, DT, AT>),
    Read(DRAMReadBundle<IT, DT>),
}

impl<IT: DAMType, DT: DAMType, AT: Clone> Peekable for AccessBundle<IT, DT, AT> {
    fn next_event(&mut self) -> crate::channel::utils::EventTime {
        match self {
            AccessBundle::Write(wr) => wr.next_event(),
            AccessBundle::Read(rd) => rd.next_event(),
        }
    }
}

// The basic DRAM handles scalar addressing
#[context_internal]
pub struct DRAM<IType: DAMType, T: DAMType, AT: DAMType> {
    config: DRAMConfig,
    datastore: Datastore<T>,
    // A rotating buffer for when each request window opens up
    request_windows: VecDeque<Time>,
    bundles: Vec<AccessBundle<IType, T, AT>>,
}

impl<IType: DAMType, T: DAMType, AT: DAMType> DRAM<IType, T, AT> {
    pub fn new(config: DRAMConfig, datastore_behavior: Behavior) -> Self {
        Self {
            config,
            datastore: Datastore::new(config.capacity, datastore_behavior),
            request_windows: VecDeque::<Time>::with_capacity(config.num_simultaneous_requests),
            bundles: vec![],
            context_info: Default::default(),
        }
    }

    pub fn fill(&mut self, mut fill_func: impl FnMut(usize) -> T) {
        for ind in 0..self.config.capacity {
            self.datastore.write(ind, fill_func(ind), Time::new(0));
        }
    }

    fn last_transfer_end_time(&self) -> Time {
        let prev_time = self.request_windows.back().copied();
        prev_time.unwrap_or_default()
    }
}

impl<I: DAMType, T: DAMType, A: DAMType> DRAM<I, T, A>
where
    Self: Context,
{
    pub fn add_writer(&mut self, drw: DRAMWriteBundle<I, T, A>) {
        drw.ack.attach_sender(self);
        drw.addr.attach_receiver(self);
        drw.data.attach_receiver(self);
        drw.request_size.attach_receiver(self);
        self.bundles.push(AccessBundle::Write(drw));
    }

    pub fn add_reader(&mut self, drr: DRAMReadBundle<I, T>) {
        drr.addr.attach_receiver(self);
        drr.data.attach_sender(self);
        drr.req_size.attach_receiver(self);
        self.bundles.push(AccessBundle::Read(drr));
    }
}

impl<IType: IndexLike, T: DAMType, AT: DAMType> Context for DRAM<IType, T, AT> {
    fn init(&mut self) {
        if self.bundles.is_empty() {
            panic!("Attempting to initialize a disconnected DRAM!");
        }
    }

    fn run(&mut self) {
        loop {
            // If all of our request windows are full, then skip forward until a window is open.
            if self.request_windows.len() == self.config.num_simultaneous_requests {
                // pop off the oldest time and advance to it.
                if let Some(next_time) = self.request_windows.pop_front() {
                    self.time.advance(next_time);
                }
            }

            // get the next event from the input streams. This implementation can fetch one
            // request at a time, but service multiple by overlapping latencies.
            let bundle_timings: Vec<EventTime> =
                self.bundles.iter_mut().map(|x| x.next_event()).collect();
            let next_event = bundle_timings
                .iter()
                .enumerate()
                .min_by_key(|(_, val)| *val);
            let event_id = match next_event {
                // If the next event is Closed, then we know that all of the channels are closed so we're done!
                Some((_, EventTime::Closed)) => {
                    // make sure that all of the event times are closed
                    assert!(bundle_timings
                        .iter()
                        .all(|time| { *time == EventTime::Closed }));
                    return;
                }
                Some((_, EventTime::Nothing(time))) => {
                    // Advance until the next time some bundle has something.
                    self.time.advance(*time + 1);
                    continue;
                }
                Some((ind, EventTime::Ready(time))) => {
                    self.time.advance(*time);
                    ind
                }
                None => unreachable!(), // This case should have been caught by the init!
            };
            let prev_transfer_time = self.last_transfer_end_time();
            match &self.bundles[event_id] {
                AccessBundle::Write(DRAMWriteBundle {
                    addr,
                    request_size,
                    data,
                    ack,
                }) => match (addr.dequeue(&self.time), request_size.dequeue(&self.time)) {
                    // This should be the only matched case since we waited for the events to show up.
                    (
                        Ok(ChannelElement {
                            time: t1,
                            data: address,
                        }),
                        Ok(ChannelElement {
                            time: t2,
                            data: write_size,
                        }),
                    ) => {
                        self.time.advance(std::cmp::max(t1, t2));
                        let start = address.to_usize();
                        let size = write_size.to_usize();

                        // For simplicity, model the entire write as one monolithic access that happens at the end.
                        // This implementation is essentially a single channel, since the data ingress occupies the entire
                        // DRAM.
                        let mut write_buffer = Vec::<T>::with_capacity(size);
                        for _ in 0..size {
                            write_buffer.push(data.dequeue(&self.time).unwrap().data);
                        }

                        // We can't start this transfer until after the previous transfer finished
                        let transfer_start_time = std::cmp::max(
                            self.time.tick() + self.config.latency,
                            prev_transfer_time,
                        );

                        // The bandwidth-constrained transfer time
                        let transfer_time =
                            u64::try_from(write_buffer.iter().map(|x| x.dam_size()).sum::<usize>())
                                .unwrap();
                        // This is when the write "actually happened"
                        let write_time = transfer_start_time + transfer_time;
                        write_buffer
                            .into_iter()
                            .enumerate()
                            .for_each(|(offset, data)| {
                                self.datastore.write(start + offset, data, write_time);
                            });

                        ack.enqueue(
                            &self.time,
                            ChannelElement {
                                time: write_time + 1,
                                data: AT::default(),
                            },
                        )
                        .unwrap();
                        // At this point, we've finished a request so we push it into the queue.
                        // This happens after the request has finished writing and after the ack has been written.
                        let next_time = std::cmp::max(self.time.tick(), write_time) + 1;
                        self.request_windows.push_back(next_time);
                    }
                    _ => unreachable!("We checked that this write bundle was ready!"),
                },
                AccessBundle::Read(DRAMReadBundle {
                    addr,
                    req_size,
                    data,
                }) => match (addr.dequeue(&self.time), req_size.dequeue(&self.time)) {
                    (
                        Ok(ChannelElement {
                            time: _,
                            data: address,
                        }),
                        Ok(ChannelElement {
                            time: _,
                            data: size,
                        }),
                    ) => {
                        let read_time = std::cmp::max(
                            self.time.tick() + self.config.latency,
                            prev_transfer_time,
                        );

                        // For the read, we should model it as a monolithic read at the START of the access.
                        let read_vals: Vec<_> = (address.to_usize()
                            ..address.to_usize() + size.to_usize())
                            .map(|ind| self.datastore.read(ind, read_time))
                            .collect();

                        let read_size: usize = read_vals.iter().map(|x| x.dam_size()).sum();
                        let transfer_time =
                            u64::try_from(read_size / self.config.bandwidth_in_bits).unwrap();
                        let read_finish_time = read_time + transfer_time;
                        let mut result_time = read_finish_time;

                        for out in read_vals {
                            result_time = std::cmp::max(self.time.tick() + 1, result_time + 1);
                            data.enqueue(&self.time, ChannelElement::new(result_time, out))
                                .unwrap();
                        }

                        self.request_windows.push_back(result_time);
                    }
                    _ => unreachable!("We checked that this read bundle was ready!"),
                },
            }
        }
    }
}

#[cfg(test)]
pub mod tests {

    use crate::{
        channel::{
            utils::{dequeue, enqueue},
            ChannelElement, Receiver,
        },
        datastructures::Time,
        simulation::{InitializationOptions, ProgramBuilder, RunOptions},
        templates::{
            datastore::Behavior,
            dram::{DRAMConfig, DRAMReadBundle, DRAMWriteBundle, DRAM},
        },
        utility_contexts::*,
    };

    #[test]
    fn test_dram_rw() {
        const TEST_SIZE: usize = 128;
        const NUM_WRITERS: usize = 4;
        const WORK_PER_WRITER: usize = TEST_SIZE / NUM_WRITERS;

        let mut dram = DRAM::<u16, u16, bool>::new(
            DRAMConfig {
                num_simultaneous_requests: 2,
                bandwidth_in_bits: 8,
                latency: Time::new(100),
                capacity: TEST_SIZE,
            },
            Behavior {
                mod_address: false,
                use_default_value: false,
            },
        );

        let mut parent = ProgramBuilder::default();
        let mut ack_channels = Vec::<Receiver<bool>>::with_capacity(NUM_WRITERS);

        (0..NUM_WRITERS).for_each(|split_ind| {
            let low = WORK_PER_WRITER * split_ind;
            let high = low + WORK_PER_WRITER;
            let (addr_send, addr_recv) = parent.bounded(128);
            let (data_send, data_recv) = parent.bounded(128);
            let (ack_send, ack_recv) = parent.bounded(128);
            let (size_send, size_recv) = parent.bounded(128);
            // Address Generator
            let addr_gen =
                GeneratorContext::new(move || [u16::try_from(low).unwrap()].into_iter(), addr_send);
            let data_gen = GeneratorContext::new(
                move || (low..high).map(|x| u16::try_from(x).unwrap()),
                data_send,
            );
            let size_gen = GeneratorContext::new(
                || [u16::try_from(WORK_PER_WRITER).unwrap()].into_iter(),
                size_send,
            );
            dram.add_writer(DRAMWriteBundle {
                addr: addr_recv,
                data: data_recv,
                request_size: size_recv,
                ack: ack_send,
            });
            ack_channels.push(ack_recv);
            parent.add_child(addr_gen);
            parent.add_child(data_gen);
            parent.add_child(size_gen);
        });

        // Create a node that waits for all of the acks to come back, and then issues reads
        let (mut rd_addr_send, rd_addr_recv) = parent.bounded(128);
        let (rd_data_send, rd_data_recv) = parent.bounded(128);
        let (req_size_send, req_size_recv) = parent.bounded(128);
        let mut read_issue = FunctionContext::new();
        ack_channels.iter_mut().for_each(|chn| {
            chn.attach_receiver(&read_issue);
        });
        rd_addr_send.attach_sender(&read_issue);
        read_issue.set_run(move |time| {
            ack_channels.iter_mut().for_each(|ack| {
                dequeue(time, ack).unwrap();
            });
            let send_time = time.tick();
            enqueue(
                time,
                &mut rd_addr_send,
                ChannelElement {
                    time: send_time,
                    data: 0,
                },
            )
            .unwrap();
        });
        parent.add_child(read_issue);

        let size_issue = GeneratorContext::new(
            || [u16::try_from(TEST_SIZE).unwrap()].into_iter(),
            req_size_send,
        );

        parent.add_child(size_issue);

        dram.add_reader(DRAMReadBundle {
            addr: rd_addr_recv,
            req_size: req_size_recv,
            data: rd_data_send,
        });

        let checker = CheckerContext::new(
            || (0..TEST_SIZE).map(|x| u16::try_from(x).unwrap()),
            rd_data_recv,
        );
        parent.add_child(checker);
        parent.add_child(dram);

        dbg!("Finished Setup!");

        parent
            .initialize(InitializationOptions::default())
            .unwrap()
            .run(RunOptions::default());
    }
}
