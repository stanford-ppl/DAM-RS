use dam_core::identifier::Identifier;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::events::{ChannelEvent, ReceiverEvent, SendEvent};
use super::ChannelID;

static DEADLOCK_DETECTOR: Lazy<Arc<Mutex<DeadlockDetector>>> = Lazy::new(|| {
    let dd = DeadlockDetector::new();
    let m = Mutex::new(dd);
    Arc::new(m)
});

pub fn register_event(event: ChannelEvent) {
    let mut dd = DEADLOCK_DETECTOR.lock().unwrap();
    dd.handle_event(event);
}

enum ContextStatus {
    Free,
    Blocked,
}

enum ChannelStatus {
    Open,
    SendClosed,
    ReceiveClosed,
}

pub struct DeadlockDetector {
    num_blocked: usize,
    context_statuses: HashMap<Identifier, ContextStatus>,
    num_channels: HashMap<Identifier, usize>,
    channel_statuses: HashMap<ChannelID, ChannelStatus>,
    /// Maps channels to (sender, receiver) pairs
    channel_users: HashMap<ChannelID, (Option<Identifier>, Option<Identifier>)>,
}

impl DeadlockDetector {
    fn new() -> Self {
        Self {
            num_blocked: 0,
            context_statuses: HashMap::new(),
            num_channels: HashMap::new(),
            channel_statuses: HashMap::new(),
            channel_users: HashMap::new(),
        }
    }

    fn register_sender(&mut self, chan_id: ChannelID, sender: Identifier) {
        if let Some(count) = self.num_channels.get_mut(&sender) {
            *count += 1;
        } else {
            self.num_channels.insert(sender, 1);
            self.context_statuses.insert(sender, ContextStatus::Free);
        }

        let users = self.channel_users.entry(chan_id).or_insert((None, None));
        users.0 = Some(sender);

        self.channel_statuses
            .entry(chan_id)
            .or_insert(ChannelStatus::Open);
    }

    fn register_receiver(&mut self, chan_id: ChannelID, receiver: Identifier) {
        if let Some(count) = self.num_channels.get_mut(&receiver) {
            *count += 1;
        } else {
            self.num_channels.insert(receiver, 1);
            self.context_statuses.insert(receiver, ContextStatus::Free);
        }

        let users = self.channel_users.entry(chan_id).or_insert((None, None));
        users.1 = Some(receiver);

        self.channel_statuses
            .entry(chan_id)
            .or_insert(ChannelStatus::Open);
    }

    fn get_sender(&self, chan_id: &ChannelID) -> Identifier {
        let users = self
            .channel_users
            .get(chan_id)
            .expect("channel should be recorded in channel_users");
        users.0.expect("sender for channel should not be None")
    }

    fn get_receiver(&self, chan_id: &ChannelID) -> Identifier {
        let users = self
            .channel_users
            .get(chan_id)
            .expect("channel should be recorded in channel_users");
        users.1.expect("sender for channel should not be None")
    }

    fn handle_send(&mut self, event: SendEvent) {
        match event {
            SendEvent::AttachSender(id, sender) => self.register_sender(id, sender),
            SendEvent::EnqueueStart(id) => {
                let status = self
                    .channel_statuses
                    .get(&id)
                    .expect("cannot find channel, likely enqueuing to a closed channel");
                if let ChannelStatus::ReceiveClosed = status {
                    panic!("enqueuing to a channel with receiver closed");
                }

                let sender = self.get_sender(&id);
                let sender_status = self
                    .context_statuses
                    .get_mut(&sender)
                    .expect("sender status not recorded");
                *sender_status = ContextStatus::Blocked;

                self.num_blocked += 1;
                if self.num_blocked == self.context_statuses.len() {
                    println!("all contexts blocked, potential deadlock");
                }
            }
            SendEvent::EnqueueFinish(id) => {}
            SendEvent::Cleanup(id) => {}
            _ => {}
        }
    }

    fn handle_receive(&mut self, event: ReceiverEvent) {
        match event {
            ReceiverEvent::AttachReceiver(id, receiver) => self.register_receiver(id, receiver),
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
