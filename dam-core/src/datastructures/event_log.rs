use std::{fs::File, io::BufWriter, time::Instant};

use std::io::Write;

use super::unrolled_linked_list::UnrolledLinkedList;

#[derive(Debug)]
struct EventWrapper<EventType> {
    time: Instant,
    event: EventType,
}

#[derive(Debug)]
pub enum FlushBehavior {
    Never,
    Periodic(usize, BufWriter<File>),
}

#[derive(Debug)]
pub struct EventLog<EventType> {
    log: UnrolledLinkedList<EventWrapper<EventType>>,
    start: Instant,
    flushbehavior: FlushBehavior,
    counter: usize,
}

impl<T: std::fmt::Debug> EventLog<T> {
    pub fn new(flushbehavior: FlushBehavior) -> Self {
        Self {
            log: Default::default(),
            start: Instant::now(),
            flushbehavior,
            counter: 0,
        }
    }

    pub fn push(&mut self, event: T) {
        self.log.push(EventWrapper {
            time: Instant::now(),
            event,
        });
        self.counter += 1;
        match self.flushbehavior {
            FlushBehavior::Periodic(cnt, _) if self.counter >= cnt => {
                self.flush();
            }
            _ => {}
        }
    }

    fn flush(&mut self) {
        let log: UnrolledLinkedList<EventWrapper<T>> = std::mem::take(&mut self.log);
        let start = self.start;
        let output = log
            .into_iter()
            .map(move |event| (event.time - start, event.event));
        match &mut self.flushbehavior {
            FlushBehavior::Never => unreachable!(),
            FlushBehavior::Periodic(_, file_out) => {
                for (time, event) in output {
                    writeln!(file_out, "{:?}: {:?}", time, event).unwrap();
                }
            }
        }
    }
}

impl<T: std::fmt::Debug> Default for EventLog<T> {
    fn default() -> Self {
        Self::new(FlushBehavior::Never)
    }
}
