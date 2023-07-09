use std::sync::{Arc, Condvar, Mutex};

use crate::time::{AtomicTime, Time};

use super::ParentView;

enum Signal {
    Done(Time),     // Immediately finished.
    Later(Condvar), // Check back later
}

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

#[derive(Clone, Default, Debug)]
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
                // If the signal time is in the present or past,
                signal.how.notify_one();
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
            let cvar = Arc::new(Condvar::new());
            signal_buffer.push(SignalElement {
                when,
                how: cvar.clone(),
            });
            let _ = cvar.wait_while(signal_buffer, |_| self.under.time.load() < when);
            return self.under.time.load();
        }
    }

    fn tick_lower_bound(&self) -> Time {
        self.under.time.load()
    }
}

// Private bookkeeping constructs
#[derive(Debug)]
struct SignalElement {
    when: Time,
    how: Arc<Condvar>,
}

// Encapsulates the callback backlog and the current tick info to make BasicContextView work.
#[derive(Default, Debug)]
struct TimeInfo {
    time: AtomicTime,
    signal_buffer: Mutex<Vec<SignalElement>>,
}
