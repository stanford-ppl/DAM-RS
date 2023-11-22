use std::sync::{Arc, Mutex};

use dam_macros::event_type_internal;
use linkme::distributed_slice;
use serde::{Deserialize, Serialize};

use crate::{
    datastructures::*,
    logging::{log_event, registry::METRICS, update_ticks, LogEvent},
};

use super::ContextView;

#[event_type_internal]
#[derive(Serialize, Deserialize, Debug)]
enum TimeEvent {
    Finish(Time),
}

/// The basic "owned" time construct.
/// All time mutations should be performed through a TimeManager.
#[derive(Default, Debug, Clone)]
pub struct TimeManager {
    underlying: Arc<TimeInfo>,
}

impl TimeManager {
    /// Constructs a new owned time.
    pub fn new() -> TimeManager {
        TimeManager {
            underlying: Arc::new(TimeInfo::default()),
        }
    }

    /// Constructs a [super::BasicContextView] of the owned time.
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

impl TimeManager {
    /// Increments time by a non-negative number of cycles.
    pub fn incr_cycles(&self, incr: u64) {
        self.underlying.time.incr_cycles(incr);
        self.scan_and_write_signals();
    }

    /// Advances to a new time. If the new time is in the past, this is a no-op.
    pub fn advance(&self, new: Time) {
        if self.underlying.time.try_advance(new) {
            self.scan_and_write_signals();
        }
    }

    fn scan_and_write_signals(&self) {
        let mut signal_buffer = self.underlying.signal_buffer.lock().unwrap();
        let tlb = self.underlying.time.load();
        // Log the updated time
        update_ticks(tlb);
        signal_buffer.retain(|signal| {
            if signal.when <= tlb {
                signal.thread.unpark();
                false
            } else {
                true
            }
        });

        drop(signal_buffer);
    }

    /// Reads the current time.
    /// Since this is only available on the owning context, it does not need to be ordered w.r.t. anything else.
    pub fn tick(&self) -> Time {
        self.underlying.time.load_relaxed()
    }

    /// Explicitly advances the context to infinite time.
    /// This is useful if we don't want to wait for `Drop` to trigger.
    pub fn cleanup(&mut self) {
        self.underlying.time.set_infinite();
        let _ = log_event(&TimeEvent::Finish(self.underlying.time.load()));
        self.scan_and_write_signals();
    }
}

/// A simple view of a "primitive" context's time.
#[derive(Clone)]
pub struct BasicContextView {
    under: Arc<TimeInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
enum ContextViewEvent {
    WaitUntil(Time),
    Park,
    Unpark,
}

impl LogEvent for ContextViewEvent {
    const NAME: &'static str = "ContextViewEvent";
}

#[distributed_slice(METRICS)]
static CONTEXT_EVENT: &'static str = ContextViewEvent::NAME;

impl ContextView for BasicContextView {
    fn wait_until(&self, when: Time) -> Time {
        let _ = log_event(&ContextViewEvent::WaitUntil(when));

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

            let _ = log_event(&ContextViewEvent::Park);

            while cur_time < when {
                // Park is Acquire, so the load can be relaxed
                std::thread::park();
                cur_time = self.under.time.load_relaxed();
            }
            let _ = log_event(&ContextViewEvent::Unpark);

            return self.under.time.load();
        }
    }

    fn tick_lower_bound(&self) -> Time {
        self.under.time.load()
    }
}

/// Registers a waking callback to a TimeManager.
/// This is used to implement wait_until on [BasicContextView]s
#[derive(Debug, Clone)]
struct SignalElement {
    when: Time,
    thread: std::thread::Thread,
}

/// Encapsulates the callback backlog and the current tick info to make BasicContextView work.
#[derive(Default, Debug)]
struct TimeInfo {
    time: AtomicTime,
    signal_buffer: Mutex<Vec<SignalElement>>,
}
