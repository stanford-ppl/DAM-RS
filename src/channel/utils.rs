use super::*;
use crate::types::DAMType;

use std::cmp::Ordering;

/// Shim for recv.dequeue
#[deprecated(
    since = "0.1.0",
    note = "This should be replaced in favor of the receiver.dequeue operation."
)]
pub fn dequeue<T: DAMType>(
    manager: &mut TimeManager,
    recv: &mut Receiver<T>,
) -> Result<ChannelElement<T>, DequeueError> {
    recv.dequeue(manager)
}

/// Shim for recv.peek_next
#[deprecated(
    since = "0.1.0",
    note = "This should be replaced in favor of the receiver.peek_next operation."
)]
pub fn peek_next<T: DAMType>(
    manager: &mut TimeManager,
    recv: &mut Receiver<T>,
) -> Result<ChannelElement<T>, DequeueError> {
    recv.peek_next(manager)
}

/// Shim for sender.enqueue
#[deprecated(
    since = "0.1.0",
    note = "This should be replaced in favor of the sender.enqueue operation."
)]
pub fn enqueue<T: DAMType>(
    manager: &mut TimeManager,
    send: &mut Sender<T>,
    data: ChannelElement<T>,
) -> Result<(), EnqueueError> {
    send.enqueue(manager, data)
}

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
    fn next_event(&mut self) -> EventTime;
}

impl Peekable for EventTime {
    fn next_event(&mut self) -> EventTime {
        *self
    }
}

impl<T: DAMType> Peekable for Receiver<T> {
    fn next_event(&mut self) -> EventTime {
        match self.peek() {
            PeekResult::Closed => EventTime::Closed,
            PeekResult::Something(time) => EventTime::Ready(time.time),
            PeekResult::Nothing(time) if time.is_infinite() => EventTime::Closed,
            PeekResult::Nothing(time) => EventTime::Nothing(time),
        }
    }
}

impl<T: Peekable> Peekable for dyn Iterator<Item = &mut T> {
    fn next_event(&mut self) -> EventTime {
        let events = self.map(|thing| thing.next_event());
        events.max().unwrap_or(EventTime::Closed)
    }
}
