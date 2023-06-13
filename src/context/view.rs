use std::sync::{Arc, RwLock};

use crossbeam::channel;

use crate::time::Time;

pub trait ContextView: Send + Sync {
    fn signal_when(&self, when: Time) -> channel::Receiver<Time>;
    fn tick_lower_bound(&self) -> Time;
}

#[derive(Clone, Default, Debug)]
pub struct TimeManager {
    arc: Arc<RwLock<TimeInfo>>,
}

impl TimeManager {
    pub fn new() -> TimeManager {
        TimeManager {
            arc: Arc::new(RwLock::default()),
        }
    }

    pub fn view(&self) -> BasicContextView {
        BasicContextView {
            under: self.arc.clone(),
        }
    }
}

impl TimeManager {
    pub fn incr_cycles<T>(&mut self, incr: T)
    where
        Time: std::ops::AddAssign<T>,
    {
        self.arc.write().unwrap().time += incr;
        self.scan_and_write_signals();
    }

    pub fn advance(&mut self, new: Time) {
        {
            let mut write = self.arc.write().unwrap();
            if write.time > new {
                return;
            }
            write.time = new;
        }
        self.scan_and_write_signals();
    }

    fn scan_and_write_signals(&mut self) {
        let tlb = self.arc.tick_lower_bound();
        let mut write = self.arc.write().unwrap();
        write.signal_buffer.retain(|signal| {
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
        self.arc.tick_lower_bound()
    }

    pub fn cleanup(&mut self) {
        self.advance(Time::infinite());
    }
}

pub trait TickLowerBound {
    fn tick_lower_bound(&self) -> Time;
}

impl TickLowerBound for Arc<RwLock<TimeInfo>> {
    fn tick_lower_bound(&self) -> Time {
        return self.read().unwrap().time;
    }
}

#[derive(Clone)]
pub struct BasicContextView {
    under: Arc<RwLock<TimeInfo>>,
}

impl ContextView for BasicContextView {
    fn signal_when(&self, when: Time) -> channel::Receiver<Time> {
        let (tx, rx) = channel::bounded::<Time>(1);

        // Check time first. Since time is non-decreasing, if this cond is true, then it's always true.
        let cur_time = self.under.tick_lower_bound();
        if cur_time >= when {
            tx.send(cur_time).unwrap();
            rx
        } else {
            let mut write = self.under.write().unwrap();
            if write.time >= when {
                tx.send(write.time).unwrap();
            } else {
                write.signal_buffer.push(SignalElement { when, how: tx })
            }
            rx
        }
    }

    fn tick_lower_bound(&self) -> Time {
        self.under.tick_lower_bound()
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
    time: Time,
    signal_buffer: Vec<SignalElement>,
}
