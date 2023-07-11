use std::sync::{atomic::AtomicBool, Arc, Mutex};

use crate::time::{AtomicTime, Time};

use super::ContextView;

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

impl TimeManager {
    pub fn incr_cycles(&mut self, incr: u64) {
        // self.log.push(TimeEvent::Incr(incr));
        self.underlying.time.incr_cycles(incr);
        self.scan_and_write_signals();
    }

    pub fn advance(&mut self, new: Time) {
        if self.underlying.time.try_advance(new) {
            // self.log.push(TimeEvent::Advance(new));
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
            // self.log.push(TimeEvent::ScanAndWrite(released));
        }
    }

    pub fn tick(&self) -> Time {
        self.underlying.time.load()
    }

    pub fn cleanup(&mut self) {
        self.underlying.time.set_infinite();
        // self.log
        //     .push(TimeEvent::Finish(self.underlying.time.load()));
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
