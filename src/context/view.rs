use std::sync::{Arc, RwLock};

use crossbeam::channel;

use crate::time::{AtomicTime, Time};

use super::ParentView;

#[enum_delegate::register]
pub trait ContextView {
    fn signal_when(&self, when: Time) -> channel::Receiver<Time>;
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
        let mut signal_buffer = self.underlying.signal_buffer.write().unwrap();
        let tlb = self.underlying.time.load();
        signal_buffer.retain(|signal| {
            if signal.when <= tlb {
                // If the signal time is in the present or past,
                let _ = signal.how.send(tlb);
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
    fn signal_when(&self, when: Time) -> channel::Receiver<Time> {
        let (tx, rx) = channel::bounded::<Time>(1);

        // Check time first. Since time is non-decreasing, if this cond is true, then it's always true.
        let cur_time = self.under.time.load();
        if cur_time >= when {
            tx.send(cur_time).unwrap();
            rx
        } else {
            let mut signal_buffer = self.under.signal_buffer.write().unwrap();
            let cur_time = self.under.time.load();
            if cur_time >= when {
                tx.send(cur_time).unwrap();
            } else {
                signal_buffer.push(SignalElement { when, how: tx })
            }
            rx
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
    how: channel::Sender<Time>,
}

// Encapsulates the callback backlog and the current tick info to make BasicContextView work.
#[derive(Default, Debug)]
struct TimeInfo {
    time: AtomicTime,
    signal_buffer: RwLock<Vec<SignalElement>>,
}
