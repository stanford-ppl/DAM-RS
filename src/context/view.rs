use std::{
    sync::{atomic::AtomicBool, Arc, Mutex},
    thread::Thread,
};

use crate::event_log::EventLog;

use crate::time::{AtomicTime, Time};

use super::ParentView;

#[enum_delegate::register]
pub trait ContextView {
    fn wait_until(&self, when: Time) -> Time;
    fn tick_lower_bound(&self) -> Time;
}

#[enum_delegate::implement(ContextView)]
pub enum TimeView {
    BasicContextView(BasicContextView),
    ParentView(ParentView),
}

#[derive(Debug, Clone, Copy)]
enum TimeEvents {
    Init,
}

#[derive(Clone, Default, Debug)]
pub struct TimeManager {
    underlying: Arc<TimeInfo>,
    log: EventLog<TimeEvents>,
}

impl TimeManager {
    pub fn new() -> TimeManager {
        TimeManager {
            underlying: Arc::new(TimeInfo::default()),
            log: Default::default(),
        }
    }

    pub fn view(&self) -> BasicContextView {
        BasicContextView {
            under: self.underlying.clone(),
        }
    }
}

impl TimeManager {
    pub fn incr_cycles(&mut self, incr: u64) {
        self.underlying.time.incr_cycles(incr);
        self.scan_and_write_signals();
    }

    pub fn advance(&mut self, new: Time) {
        if self.underlying.time.try_advance(new) {
            self.scan_and_write_signals();
        }
    }

    fn scan_and_write_signals(&mut self) {
        let mut signal_buffer = self.underlying.signal_buffer.lock().unwrap();
        let tlb = self.underlying.time.load();
        signal_buffer.retain(|signal| {
            if signal.when <= tlb {
                signal
                    .done
                    .store(true, std::sync::atomic::Ordering::Release);
                signal.thread.unpark();
                false
            } else {
                true
            }
        })
    }

    pub fn tick(&self) -> Time {
        self.underlying.time.load()
    }

    pub fn cleanup(&mut self) {
        self.underlying.time.set_infinite();
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

#[derive(Debug)]
struct Signal {
    thread: Thread,
    done: AtomicBool,
}

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
