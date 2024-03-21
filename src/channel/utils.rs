use super::*;
use crate::types::DAMType;

use std::cmp::Ordering;

/// When a channel will have a meaningful event. This is useful when it is possible to read/write to one of many channels
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum EventTime {
    /// Event will happen at given timestamp
    Ready(Time),

    /// Event won't happen up to timestamp -- useful if we wish to determine the first event in a list.
    Nothing(Time),

    /// Event will never happen, roughly equivalent to Nothing(Infinity)
    Closed,
}

impl EventTime {
    fn key(&self) -> Time {
        match self {
            EventTime::Ready(time) => *time,
            EventTime::Nothing(time) => *time + 1,
            EventTime::Closed => Time::infinite(),
        }
    }
}

impl Ord for EventTime {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(&other.key())
    }
}

impl PartialOrd for EventTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Objects which can can have their next event queried -- mostly used for channel-type objects.
pub trait Peekable {
    /// Gets a timestamp of the next possible event.
    fn next_event(self) -> EventTime;
}

impl Peekable for EventTime {
    fn next_event(self) -> EventTime {
        self
    }
}

impl<T: DAMType> Peekable for &Receiver<T> {
    fn next_event(self) -> EventTime {
        match self.peek() {
            PeekResult::Closed => EventTime::Closed,
            PeekResult::Something(time) => EventTime::Ready(time.time),
            PeekResult::Nothing(time) if time.is_infinite() => EventTime::Closed,
            PeekResult::Nothing(time) => EventTime::Nothing(time),
        }
    }
}

/// A utility structure whose next event is dictated by ALL sub-events being ready
pub struct EventAll<T> {
    under: T,
}

impl<T, It> Peekable for EventAll<It>
where
    It: Iterator<Item = T>,
    T: Peekable,
{
    fn next_event(self) -> EventTime {
        let events = self.under.map(|thing| thing.next_event());
        events.max().unwrap_or(EventTime::Closed)
    }
}

/// A utility structure whose next event is dictated by ANY sub-event being ready
pub struct EventAny<T> {
    under: T,
}

impl<T, It> Peekable for EventAny<It>
where
    It: Iterator<Item = T>,
    T: Peekable,
{
    fn next_event(self) -> EventTime {
        let events = self.under.map(|thing| thing.next_event());
        events.min().unwrap_or(EventTime::Closed)
    }
}

/// Shim trait for creating [EventAny] and [EventAll] from iterators
pub trait EventCollection<T> {
    /// Shim for creating [EventAll]
    fn all_events(self) -> EventAll<T>;

    /// Shim for creating [EventAny]
    fn any_event(self) -> EventAny<T>;
}

impl<II, T> EventCollection<II::IntoIter> for II
where
    II: IntoIterator<Item = T>,
    T: Peekable,
{
    fn all_events(self) -> EventAll<II::IntoIter> {
        EventAll {
            under: self.into_iter(),
        }
    }

    fn any_event(self) -> EventAny<II::IntoIter> {
        EventAny {
            under: self.into_iter(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        simulation::ProgramBuilder,
        utility_contexts::{random_trace, FunctionContext, TraceContext},
    };

    use super::{EventTime, Peekable};

    /// Puts stuff in a channel and checks when it's available.
    #[test]
    fn test_receiver() {
        let mut ctx = ProgramBuilder::default();
        let (snd, rcv) = ctx.unbounded();
        ctx.add_child(TraceContext::new(
            // (Value, Time) pairs
            || random_trace(1024, 0, 16),
            snd,
        ));

        let mut fc = FunctionContext::default();
        rcv.attach_receiver(&fc);
        fc.set_run(move |time| loop {
            let pre_peek = rcv.next_event();
            let peek_next = rcv.peek_next(time);
            let post_peek = rcv.next_event();
            match (pre_peek, post_peek, peek_next) {
                (prev, EventTime::Ready(post_time), Ok(ce)) if post_time == ce.time => {
                    match prev {
                        EventTime::Ready(pre_time) if pre_time == post_time => {}
                        EventTime::Nothing(pre_time) if pre_time < post_time => {}
                        _ => {
                            panic!("Pre-peek event: {prev:?} was not compatible with post-peek {post_peek:?}");
                        }
                    };
                    // Pop the value off the stream
                    let _ = rcv.dequeue(time);
                },

                (_, EventTime::Ready(post_time), Ok(ce)) => {
                    // Mismatched case
                    panic!(
                        "Mismatched ready and post times: {post_time:?} != {:?}",
                        ce.time
                    );
                }

                (EventTime::Nothing(_), EventTime::Closed, Err(_)) => {
                    // Originally was just nothing, but now we've found out that it's closed.
                    // For completeness, delegate it to the next iteration
                    // Otherwise it's possible to never test the next case.
                    continue;
                }
                (EventTime::Closed, EventTime::Closed, Err(_)) => {
                    // Next events both said closed, was closed
                    return;
                }

                (_, _, pn) => {
                    panic!("Incompatible state found: Pre [{pre_peek:?}], Post [{post_peek:?}], Received [{pn:?}]");
                }
            }
        });
        ctx.add_child(fc);
        ctx.initialize(Default::default())
            .unwrap()
            .run(Default::default());
    }
}
