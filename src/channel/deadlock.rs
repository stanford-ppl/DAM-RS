use once_cell::sync::Lazy;
use std::sync::{Arc, Mutex};

use super::events::{ChannelEvent, ReceiverEvent, SendEvent};

static DEADLOCK_DETECTOR: Lazy<Arc<Mutex<DeadlockDetector>>> = Lazy::new(|| {
    let dd = DeadlockDetector::new();
    let m = Mutex::new(dd);
    Arc::new(m)
});

pub fn register_event(event: ChannelEvent) {
    let mut dd = DEADLOCK_DETECTOR.lock().unwrap();
    dd.handle_event(event);
}

pub struct DeadlockDetector {}

impl DeadlockDetector {
    fn new() -> Self {
        Self {}
    }

    fn handle_send(&mut self, event: SendEvent) {
        match event {
            SendEvent::AttachSender(id, sender) => {}
            SendEvent::EnqueueStart(id) => {}
            SendEvent::EnqueueFinish(id) => {}
            SendEvent::Cleanup(id) => {}
            _ => {}
        }
    }

    fn handle_receive(&mut self, event: ReceiverEvent) {
        match event {
            ReceiverEvent::AttachReceiver(id, receiver) => {}
            ReceiverEvent::PeekNextStart(id) => {}
            ReceiverEvent::PeekNextFinish(id) => {}
            ReceiverEvent::DequeueStart(id) => {}
            ReceiverEvent::DequeueFinish(id) => {}
            ReceiverEvent::Cleanup(id) => {}
            _ => {}
        }
    }

    fn handle_event(&mut self, event: ChannelEvent) {
        match event {
            ChannelEvent::SendEvent(send) => self.handle_send(send),
            ChannelEvent::ReceiverEvent(receiver) => self.handle_receive(receiver),
        }
    }
}
