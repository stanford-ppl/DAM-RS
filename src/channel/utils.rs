use std::cmp::max;

use crate::context::view::TimeManager;
use crate::time::Time;
use crate::types::DAMType;

use super::ChannelElement;
use super::DequeueError;
use super::EnqueueError;
use super::Receiver;
use super::Recv;
use super::Sender;

use std::cmp::Ordering;

pub fn dequeue<T: Copy>(
    manager: &mut TimeManager,
    recv: &mut Receiver<T>,
) -> Result<ChannelElement<T>, DequeueError> {
    loop {
        let v = recv.recv();
        match v {
            Recv::Nothing(time) => manager.advance(time + 1), // Nothing here, so tick forward until there might be
            Recv::Closed => return Err(DequeueError {}), // Channel is closed, so let the dequeuer know
            Recv::Something(stuff) => return Ok(stuff),
        }
    }
}

pub fn enqueue<T: Copy>(
    manager: &mut TimeManager,
    send: &mut Sender<T>,
    data: ChannelElement<T>,
) -> Result<(), EnqueueError> {
    loop {
        let send_data = ChannelElement::new(max(data.time, manager.tick()), data.data);
        let v = send.send(send_data);
        match v {
            Ok(()) => return Ok(()),
            Err(time) if time.is_infinite() => {
                return Err(EnqueueError {});
            }
            Err(time) => {
                manager.advance(time + 1);
            }
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum EventTime {
    Ready(Time),
    Nothing(Time),
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

pub trait Peekable {
    fn next_event(&mut self) -> EventTime;
}

impl<T: DAMType> Peekable for Receiver<T> {
    fn next_event(&mut self) -> EventTime {
        match self.peek() {
            Recv::Closed => EventTime::Closed,
            Recv::Something(time) => EventTime::Ready(time.time),
            Recv::Nothing(time) => EventTime::Nothing(time),
        }
    }
}

impl<T: Peekable> Peekable for dyn Iterator<Item = &mut T> {
    fn next_event(&mut self) -> EventTime {
        let events = self.map(|thing| thing.next_event());
        events.max().unwrap_or(EventTime::Closed)
    }
}
