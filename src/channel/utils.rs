use crate::types::DAMType;
use dam_core::time::Time;
use dam_core::TimeManager;

use super::*;

use std::cmp::Ordering;

pub struct RecvBundle<T: DAMType> {
    receivers: Vec<Receiver<T>>,
}

pub struct SendBundle<T: DAMType> {
    senders: Vec<Sender<T>>,
}

impl<T: DAMType> Peekable for RecvBundle<T> {
    fn next_event(&mut self) -> EventTime {
        let events = self.receivers.iter_mut().map(|recv| recv.next_event());
        events.max().unwrap_or(EventTime::Closed)
    }
}

impl<T: DAMType> RecvBundle<T> {
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

pub fn dequeue<T: DAMType>(
    manager: &mut TimeManager,
    recv: &mut Receiver<T>,
) -> Result<ChannelElement<T>, DequeueError> {
    match recv.dequeue(manager) {
        Recv::Something(ce) => Ok(ce),
        Recv::Closed => Err(DequeueError {}),
        _ => panic!("Should only ever get something or closed on dequeue"),
    }
}

pub fn peek_next<T: DAMType>(
    manager: &mut TimeManager,
    recv: &mut Receiver<T>,
) -> Result<ChannelElement<T>, DequeueError> {
    match recv.peek_next(manager) {
        Recv::Something(ce) => Ok(ce),
        Recv::Closed => Err(DequeueError {}),
        Recv::Unknown | Recv::Nothing(_) => unreachable!(),
    }
}

pub fn dequeue_bundle<T: DAMType>(
    manager: &mut TimeManager,
    bundles: &mut [RecvBundle<T>],
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
        None => Err(DequeueError {}),
    }
}

pub fn enqueue<T: DAMType>(
    manager: &mut TimeManager,
    send: &mut Sender<T>,
    data: ChannelElement<T>,
) -> Result<(), EnqueueError> {
    send.enqueue(manager, data)
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
        *self
    }
}

impl<T: DAMType> Peekable for Receiver<T> {
    fn next_event(&mut self) -> EventTime {
        match self.peek() {
            Recv::Closed => EventTime::Closed,
            Recv::Something(time) => EventTime::Ready(time.time),
            Recv::Nothing(time) if time.is_infinite() => EventTime::Closed,
            Recv::Nothing(time) => EventTime::Nothing(time),
            Recv::Unknown => panic!("Can't get an Unknown on a peek!"),
        }
    }
}

impl<T: Peekable> Peekable for dyn Iterator<Item = &mut T> {
    fn next_event(&mut self) -> EventTime {
        let events = self.map(|thing| thing.next_event());
        events.max().unwrap_or(EventTime::Closed)
    }
}
