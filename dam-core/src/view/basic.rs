use std::sync::{atomic::AtomicBool, Arc, Mutex};

use linkme::distributed_slice;
use serde::{Deserialize, Serialize};

use crate::{
    identifier::Identifier,
    log_graph::get_graph,
    metric::{LogProducer, METRICS},
    time::{AtomicTime, Time},
};

use super::ContextView;

#[derive(Serialize, Deserialize, Debug)]
enum TimeEvent {
    Incr(u64),
    Advance(Time),
    ScanAndWrite(Time, Vec<Identifier>),
    Finish(Time),
}

#[derive(Default, Debug)]
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

impl LogProducer for TimeManager {
    const LOG_NAME: &'static str = "time_manager";
}

#[distributed_slice(METRICS)]
static TIMEMANAGER_NAME: &'static str = "time_manager";

impl TimeManager {
    pub fn incr_cycles(&mut self, incr: u64) {
        Self::log(TimeEvent::Incr(incr));
        self.underlying.time.incr_cycles(incr);
        self.scan_and_write_signals();
    }

    pub fn advance(&mut self, new: Time) {
        if self.underlying.time.try_advance(new) {
            Self::log(TimeEvent::Advance(new));
            self.scan_and_write_signals();
        }
    }

    fn scan_and_write_signals(&mut self) {
        let mut signal_buffer = self.underlying.signal_buffer.lock().unwrap();
        let tlb = self.underlying.time.load();
        let mut released = Vec::new();
        signal_buffer.retain(|signal| {
            if signal.when <= tlb {
                signal
                    .done
                    .store(true, std::sync::atomic::Ordering::Release);
                signal.thread.unpark();
                released.push(signal.thread.id());
                false
            } else {
                true
            }
        });

        drop(signal_buffer);
        if !released.is_empty() {
            let graph = get_graph();
            Self::log(TimeEvent::ScanAndWrite(
                tlb,
                released
                    .into_iter()
                    .map(|thr| graph.get_identifier(thr))
                    .collect(),
            ));
        }
    }

    pub fn tick(&self) -> Time {
        self.underlying.time.load()
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

impl ContextView for BasicContextView {
    fn wait_until(&self, when: Time) -> Time {
        // Check time first. Since time is non-decreasing, if this cond is true, then it's always true.
        let cur_time = self.under.time.load();
        if cur_time >= when {
            return cur_time;
        }

        let mut signal_buffer = self.under.signal_buffer.lock().unwrap();
        let cur_time = self.under.time.load();
        if cur_time >= when {
            return cur_time;
        } else {
            let done = Arc::new(AtomicBool::new(false));
            signal_buffer.push(SignalElement {
                when,
                done: done.clone(),
                thread: std::thread::current(),
            });
            // Unlock the signal buffer
            drop(signal_buffer);

            while !done.load(std::sync::atomic::Ordering::Acquire) {
                std::thread::park();
            }

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

    done: Arc<AtomicBool>,
    thread: std::thread::Thread,
}

// Encapsulates the callback backlog and the current tick info to make BasicContextView work.
#[derive(Default, Debug)]
struct TimeInfo {
    time: AtomicTime,
    signal_buffer: Mutex<Vec<SignalElement>>,
}
