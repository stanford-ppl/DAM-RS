use crate::types::DAMType;
use dam_core::time::Time;
use dam_core::TimeManager;

use super::ChannelElement;
use super::ChannelFlavor;
use super::DequeueError;
use super::EnqueueError;
use super::Receiver;
use super::Recv;
use super::SendOptions;
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
    if let ChannelFlavor::Acyclic = recv.view_struct.flavor {
        match recv.dequeue_sync() {
            Recv::Something(ce) => {
                manager.advance(ce.time);
                return Ok(ce);
            }
            Recv::Closed => return Err(DequeueError {}),
            Recv::Nothing(_) | Recv::Unknown => {
                unreachable!();
            }
        }
    }

    // Async dequeue

    loop {
        let v = recv.recv();
        match v {
            Recv::Nothing(time) => manager.advance(time + 1), // Nothing here, so tick forward until there might be
            Recv::Closed => return Err(DequeueError {}), // Channel is closed, so let the dequeuer know
            Recv::Something(stuff) => {
                manager.advance(stuff.time);
                return Ok(stuff);
            }
            Recv::Unknown => panic!("Can't receive an Unknown!"),
        }
    }
}

pub fn peek_next<T: DAMType>(
    manager: &mut TimeManager,
    recv: &mut Receiver<T>,
) -> Result<ChannelElement<T>, DequeueError> {
    match recv.view_struct.flavor {
        super::ChannelFlavor::Unknown | super::ChannelFlavor::Cyclic => {
            peek_next_async(manager, recv)
        }
        super::ChannelFlavor::Acyclic => {
            let recv = recv.peek_next_sync();

            match recv {
                Recv::Something(data) => {
                    manager.advance(data.time);
                    Ok(data)
                }
                Recv::Closed => Err(DequeueError {}),
                _ => unreachable!(),
            }
        }
    }
}

fn peek_next_async<T: DAMType>(
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
            Recv::Unknown => panic!("Can't peek_next an unknown!"),
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

pub fn enqueue<T: DAMType>(
    manager: &mut TimeManager,
    send: &mut Sender<T>,
    data: ChannelElement<T>,
) -> Result<(), EnqueueError> {
    let mut data_copy = data.clone();
    loop {
        data_copy.update_time(manager.tick() + 1);
        let v = send.send(data_copy.clone());
        match v {
            Ok(()) => return Ok(()),
            Err(SendOptions::Never) => {
                return Err(EnqueueError {});
            }
            Err(SendOptions::CheckBackAt(time)) | Err(SendOptions::AvailableAt(time)) => {
                // Have to make sure that we're making progress
                assert!(time > manager.tick());
                manager.advance(time);
            }
            Err(SendOptions::Unknown) => {
                unreachable!("We should always know when to try again!")
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
