use std::{
    cmp::Ordering,
    sync::atomic::{AtomicBool, AtomicU64},
};

use serde::{Deserialize, Serialize};

/// An immutable timestamp.
/// The time is stored as a separate u64 and a done flag. This way a context can be marked as finished (via infinite time), but preserves the actual timestamp for logging.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Time {
    time: u64,
    done: bool,
}

impl Time {
    /// Constructs a time from a non-negative initializer
    pub fn new(time: u64) -> Self {
        Self { time, done: false }
    }

    /// Constructs an infinite timestamp.
    pub fn infinite() -> Self {
        Self {
            time: 0,
            done: true,
        }
    }

    /// Checks whether the timestamp is 'done'
    pub fn is_infinite(&self) -> bool {
        self.done
    }

    /// Marks this timestamp as infinite
    pub fn set_infinite(&mut self) {
        self.done = true;
    }

    /// Gets the underlying tick count, regardless of whether it is marked as infinite.
    pub fn time(&self) -> u64 {
        self.time
    }

    fn cmp(&self, other: &Self) -> Ordering {
        if self == other {
            Ordering::Equal
        } else if self.done {
            Ordering::Greater
        } else if other.done {
            Ordering::Less
        } else {
            self.time.cmp(&other.time)
        }
    }
}

impl PartialEq for Time {
    fn eq(&self, other: &Self) -> bool {
        if self.done && other.done {
            true
        } else if self.done != other.done {
            false
        } else {
            self.time == other.time
        }
    }
}

impl Eq for Time {}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cmp(other)
    }
}

impl std::fmt::Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_infinite() {
            write!(f, "inf {}", self.time)
        } else {
            self.time.fmt(f)
        }
    }
}

impl std::ops::Add<u64> for Time {
    type Output = Time;

    fn add(self, rhs: u64) -> Time {
        Time {
            time: self.time + rhs,
            done: self.done,
        }
    }
}

impl std::ops::Add<Time> for Time {
    type Output = Time;
    fn add(self, rhs: Time) -> Time {
        Time {
            time: self.time + rhs.time,
            done: self.done || rhs.done,
        }
    }
}

impl std::ops::Sub<u64> for Time {
    type Output = Time;
    fn sub(self, rhs: u64) -> Time {
        assert!(self.time >= rhs);
        Time {
            time: self.time - rhs,
            done: self.done,
        }
    }
}

impl std::ops::AddAssign<u64> for Time {
    fn add_assign(&mut self, rhs: u64) {
        self.time += rhs;
    }
}

impl std::ops::AddAssign<Time> for Time {
    fn add_assign(&mut self, rhs: Time) {
        self.time += rhs.time;
        self.done |= rhs.done;
    }
}

/// An atomic notion of time, used by the [crate::view::TimeManager] construct.
#[derive(Debug, Default)]
pub(crate) struct AtomicTime {
    time: AtomicU64,
    done: AtomicBool,
}

impl AtomicTime {
    const UPDATE_ORDERING: std::sync::atomic::Ordering = std::sync::atomic::Ordering::Release;

    /// Reads the underlying time with Acquire semantics, used by other threads.
    pub fn load(&self) -> Time {
        let time = self.time.load(std::sync::atomic::Ordering::Relaxed);
        let done = self.done.load(std::sync::atomic::Ordering::Acquire);
        Time { time, done }
    }

    /// Reads the underlying time with Relaxed semantics, only safe when performing optimistic operations
    /// or within the same thread.
    pub fn load_relaxed(&self) -> Time {
        let time = self.time.load(std::sync::atomic::Ordering::Relaxed);
        let done = self.done.load(std::sync::atomic::Ordering::Relaxed);
        Time { time, done }
    }

    pub fn set_infinite(&self) {
        self.done.fetch_or(true, Self::UPDATE_ORDERING);
    }

    pub fn try_advance(&self, rhs: Time) -> bool {
        let old_done = self.done.load(std::sync::atomic::Ordering::Relaxed);
        match (old_done, rhs.done) {
            (true, _) => {
                // If we're both done, then just finish.
                // If we were done, but the new time isn't, then ignore.
                false
            }
            (false, true) => {
                self.done.store(true, std::sync::atomic::Ordering::Release);
                // If we weren't done, but they were.
                true
            }
            (false, false) => {
                // If we weren't done, and neither were they.
                let old_time = self.time.load(std::sync::atomic::Ordering::Relaxed);
                if old_time < rhs.time {
                    self.time
                        .store(rhs.time, std::sync::atomic::Ordering::Release);
                    return true;
                }
                return false;
            }
        }
    }

    pub fn incr_cycles(&self, rhs: u64) {
        self.time.fetch_add(rhs, Self::UPDATE_ORDERING);
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::{max, min};

    use super::*;

    #[test]
    fn time_equality() {
        let inf0 = Time {
            time: 0,
            done: true,
        };
        let inf1 = Time {
            time: 1,
            done: true,
        };
        assert_eq!(inf0, inf1);
        assert_eq!(inf1, inf0);

        let fin0 = Time::new(0);
        assert_ne!(fin0, inf0);
        assert_ne!(inf0, fin0);

        let fin00 = Time::new(0);
        assert_eq!(fin0, fin00);
        assert_eq!(fin00, fin0);
    }

    #[test]
    fn time_cmp() {
        let inf0 = Time::infinite();
        let fin1 = Time::new(1);
        assert!(inf0 > fin1);
        assert!(fin1 < inf0);

        let fin0 = Time::new(0);
        assert!(fin0 < fin1);
        assert!(fin1 > fin0);

        assert_eq!(*min(&inf0, &fin1), fin1);
        assert_eq!(*max(&inf0, &fin1), inf0);

        assert_eq!(*min(&fin0, &fin1), fin0);
        assert_eq!(*max(&fin0, &fin1), fin1);
    }

    #[test]
    fn time_add() {
        let fin0 = Time::new(0);

        let fin42 = fin0 + 42;
        assert_eq!(fin42.time, 42);
        assert!(!fin42.done);

        let mut fin1 = Time::new(1);
        fin1 += 1;
        assert_eq!(fin1.time, 2);
    }
}
