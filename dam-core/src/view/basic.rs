use std::sync::{Arc, Mutex};

use linkme::distributed_slice;
use serde::{Deserialize, Serialize};

use crate::{
    datastructures::*,
    metric::{LogProducer, METRICS},
};

use super::ContextView;

#[derive(Serialize, Deserialize, Debug)]
enum TimeEvent {
    Incr(u64),
    Advance(Time),
    ScanAndWrite(Time, Vec<Identifier>),
    Finish(Time),
}

#[derive(Default, Debug, Clone)]
pub struct TimeManager {
    underlying: Arc<TimeInfo>,
}

impl TimeManager {
    pub fn new() -> TimeManager {
        TimeManager {
            underlying: Arc::new(TimeInfo::default()),
        }
    }

    pub fn view(&self) -> BasicContextView {
        BasicContextView {
            under: self.underlying.clone(),
        }
    }
}

impl Drop for TimeManager {
    fn drop(&mut self) {
        self.cleanup();
    }
}

impl LogProducer for TimeManager {
    const LOG_NAME: &'static str = "TimeManager";
}

#[distributed_slice(METRICS)]
static TIMEMANAGER_NAME: &'static str = "TimeManager";

impl TimeManager {
    pub fn incr_cycles(&self, incr: u64) {
        Self::log(TimeEvent::Incr(incr));
        self.underlying.time.incr_cycles(incr);
        self.scan_and_write_signals();
    }

    pub fn advance(&self, new: Time) {
        if self.underlying.time.try_advance(new) {
            Self::log(TimeEvent::Advance(new));
            self.scan_and_write_signals();
        }
    }

    fn scan_and_write_signals(&self) {
        let mut signal_buffer = self.underlying.signal_buffer.lock().unwrap();
        let tlb = self.underlying.time.load();
        #[cfg(logging)]
        let mut released = Vec::new();
        signal_buffer.retain(|signal| {
            if signal.when <= tlb {
                signal.thread.unpark();
                #[cfg(logging)]
                released.push(signal.thread.id());
                false
            } else {
                true
            }
        });

        drop(signal_buffer);
        #[cfg(logging)]
        if !released.is_empty() {
            let graph = get_registry();
            Self::log(TimeEvent::ScanAndWrite(
                tlb,
                released
                    .into_iter()
                    .map(|thr| {
                        graph
                            .get_identifier(thr)
                            .expect("Not all threads had registered identifiers!")
                    })
                    .collect(),
            ));
        }
    }

    pub fn tick(&self) -> Time {
        self.underlying.time.load_relaxed()
    }

    pub fn cleanup(&mut self) {
        self.underlying.time.set_infinite();
        Self::log(TimeEvent::Finish(self.underlying.time.load()));
        self.scan_and_write_signals();
    }
}

#[derive(Clone)]
pub struct BasicContextView {
    under: Arc<TimeInfo>,
}

impl LogProducer for BasicContextView {
    const LOG_NAME: &'static str = "BasicContextView";
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ContextViewEvent {
    WaitUntil(Time),
    Park,
    Unpark,
}

#[distributed_slice(METRICS)]
static CONTEXTVIEW_NAME: &'static str = "BasicContextView";
impl ContextView for BasicContextView {
    fn wait_until(&self, when: Time) -> Time {
        Self::log(ContextViewEvent::WaitUntil(when));

        // Check time first. Since time is non-decreasing, if this cond is true, then it's always true.
        let cur_time = self.under.time.load();
        if cur_time >= when {
            return cur_time;
        }

        let mut signal_buffer = self.under.signal_buffer.lock().unwrap();
        let mut cur_time = self.under.time.load();
        if cur_time >= when {
            return cur_time;
        } else {
            signal_buffer.push(SignalElement {
                when,
                thread: std::thread::current(),
            });
            // Unlock the signal buffer
            drop(signal_buffer);

            Self::log(ContextViewEvent::Park);

            while cur_time < when {
                // Park is Acquire, so the load can be relaxed
                std::thread::park();
                cur_time = self.under.time.load_relaxed();
            }
            Self::log(ContextViewEvent::Unpark);

            return self.under.time.load();
        }
    }

    fn tick_lower_bound(&self) -> Time {
        self.under.time.load()
    }
}

// Private bookkeeping constructs

#[derive(Debug, Clone)]
struct SignalElement {
    when: Time,
    thread: std::thread::Thread,
}

// Encapsulates the callback backlog and the current tick info to make BasicContextView work.
#[derive(Default, Debug)]
struct TimeInfo {
    time: AtomicTime,
    signal_buffer: Mutex<Vec<SignalElement>>,
}
