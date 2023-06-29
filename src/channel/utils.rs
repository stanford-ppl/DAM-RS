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

pub struct RecvBundle<T> {
    receivers: Vec<Receiver<T>>,
}

pub struct SendBundle<T> {
    senders: Vec<Sender<T>>,
}

impl<T: DAMType> Peekable for RecvBundle<T> {
    fn next_event(&mut self) -> EventTime {
        let events = self.receivers.iter_mut().map(|recv| recv.next_event());
        events.max().unwrap_or(EventTime::Closed)
    }
}

impl<T: Copy> RecvBundle<T> {
    fn dequeue(
        &mut self,
        manager: &mut TimeManager,
    ) -> Vec<Result<ChannelElement<T>, DequeueError>> {
        self.receivers
            .iter_mut()
            .map(|recv| dequeue(manager, recv))
            .collect()
    }
}

pub fn dequeue<T: Copy>(
    manager: &mut TimeManager,
    recv: &mut Receiver<T>,
) -> Result<ChannelElement<T>, DequeueError> {
    loop {
        let v = recv.recv();
        match v {
            Recv::Nothing(time) => manager.advance(time + 1), // Nothing here, so tick forward until there might be
            Recv::Closed => return Err(DequeueError {}), // Channel is closed, so let the dequeuer know
            Recv::Something(stuff) => {
                manager.advance(stuff.time);
                return Ok(stuff);
            }
        }
    }
}

pub fn peek_next<T: Copy>(
    manager: &mut TimeManager,
    recv: &mut Receiver<T>,
) -> Result<ChannelElement<T>, DequeueError> {
    loop {
        let v: Recv<T> = recv.peek();
        match v {
            Recv::Nothing(time) => manager.advance(time + 1), // Nothing here, so tick forward until there might be
            Recv::Closed => return Err(DequeueError {}), // Channel is closed, so let the dequeuer know
            Recv::Something(stuff) => {
                manager.advance(stuff.time);
                return Ok(stuff);
            }
        }
    }
}

pub fn dequeue_bundle<T: DAMType>(
    manager: &mut TimeManager,
    bundles: &mut Vec<RecvBundle<T>>,
) -> Result<(Vec<ChannelElement<T>>, usize), DequeueError> {
    let next_events = bundles.iter_mut().map(|bundle| bundle.next_event());
    let earliest_event = next_events.enumerate().min_by_key(|(_, evt)| *evt);
    match earliest_event {
        Some((ind, _)) => {
            let dequeued = bundles[ind].dequeue(manager);
            let mut result = Vec::<ChannelElement<T>>::with_capacity(dequeued.len());
            for sub_result in dequeued {
                match sub_result {
                    Ok(elem) => result.push(elem),
                    Err(e) => return Err(e),
                }
            }
            Ok((result, ind))
        }
        None => return Err(DequeueError {}),
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

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
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

impl Peekable for EventTime {
    fn next_event(&mut self) -> EventTime {
        return *self;
    }
}

impl<T: Copy> Peekable for Receiver<T> {
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
