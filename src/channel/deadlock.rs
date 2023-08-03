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
    println!("{:?}", event);
    let mut dd = DEADLOCK_DETECTOR.lock().unwrap();
    dd.handle_event(event);
}

enum ContextStatus {
    Free,
    Blocked,
}

#[derive(PartialEq)]
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

    fn register_context(&mut self, chan_id: ChannelID, context: Identifier, is_sender: bool) {
        if let Some(count) = self.num_channels.get_mut(&context) {
            *count += 1;
        } else {
            self.num_channels.insert(context, 1);
            self.context_statuses.insert(context, ContextStatus::Free);
        }

        let users = self.channel_users.entry(chan_id).or_insert((None, None));
        if is_sender {
            users.0 = Some(context);
        } else {
            users.1 = Some(context);
        }

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

    fn get_channel_status(&self, chan_id: &ChannelID) -> &ChannelStatus {
        self.channel_statuses
            .get(chan_id)
            .expect("cannot find channel, likely enqueuing to a closed channel")
    }

    fn get_channel_status_mut(&mut self, chan_id: &ChannelID) -> &mut ChannelStatus {
        self.channel_statuses
            .get_mut(chan_id)
            .expect("cannot find channel, likely enqueuing to a closed channel")
    }

    fn get_sender_status_mut(&mut self, sender: &Identifier) -> &mut ContextStatus {
        self.context_statuses
            .get_mut(sender)
            .expect("sender status not recorded")
    }

    fn enqueue_channel_status_check(&self, chan_id: &ChannelID) {
        let status = self.get_channel_status(chan_id);
        if let ChannelStatus::ReceiveClosed = status {
            panic!("enqueuing to a channel with receiver closed");
        }
    }

    fn dequeue_channel_status_check(&self, chan_id: &ChannelID) {
        self.get_channel_status(chan_id);
    }

    fn set_context_status(&mut self, context: &Identifier, status: ContextStatus) {
        let context_status = self.get_sender_status_mut(context);
        *context_status = status;
    }

    fn get_num_channels_mut(&mut self, context: &Identifier) -> &mut usize {
        self.num_channels
            .get_mut(context)
            .expect("sender's number of channel should be recorded")
    }

    fn deadlock_check(&self) {
        println!("{}/{}", self.num_blocked, self.context_statuses.len());
        if self.num_blocked == self.context_statuses.len() {
            println!("all contexts blocked, potential deadlock");
        }
    }

    fn cleanup_context(&mut self, chan_id: &ChannelID, is_sender: bool) {
        let status = self.get_channel_status_mut(chan_id);
        let remove_criterion = if is_sender {
            ChannelStatus::ReceiveClosed
        } else {
            ChannelStatus::SendClosed
        };

        if remove_criterion == *status {
            self.channel_statuses.remove(chan_id);
        } else {
            *status = if is_sender {
                ChannelStatus::SendClosed
            } else {
                ChannelStatus::ReceiveClosed
            };
        }

        let context = self.get_sender(chan_id);
        let channels = self.get_num_channels_mut(&context);
        *channels -= 1;
        if *channels == 0 {
            self.context_statuses.remove(&context);
        }
    }

    fn handle_send(&mut self, event: SendEvent) {
        match event {
            SendEvent::AttachSender(id, sender) => {
                self.register_context(id, sender, true);
            }
            SendEvent::EnqueueStart(id) => {
                self.enqueue_channel_status_check(&id);

                let sender = self.get_sender(&id);
                self.set_context_status(&sender, ContextStatus::Blocked);

                self.num_blocked += 1;
                self.deadlock_check();
            }
            SendEvent::EnqueueFinish(id) => {
                self.enqueue_channel_status_check(&id);
                let sender = self.get_sender(&id);
                self.set_context_status(&sender, ContextStatus::Free);
                self.num_blocked -= 1;
            }
            SendEvent::Cleanup(id) => {
                self.cleanup_context(&id, true);
            }
            _ => {}
        }
    }

    fn handle_receive(&mut self, event: ReceiverEvent) {
        match event {
            ReceiverEvent::AttachReceiver(id, receiver) => {
                self.register_context(id, receiver, false)
            }
            ReceiverEvent::PeekNextStart(id) | ReceiverEvent::DequeueStart(id) => {
                self.dequeue_channel_status_check(&id);
                let receiver = self.get_receiver(&id);
                self.set_context_status(&receiver, ContextStatus::Blocked);
                self.num_blocked += 1;
                self.deadlock_check();
            }
            ReceiverEvent::PeekNextFinish(id) | ReceiverEvent::DequeueFinish(id) => {
                self.dequeue_channel_status_check(&id);
                let receiver = self.get_receiver(&id);
                self.set_context_status(&receiver, ContextStatus::Free);
                self.num_blocked -= 1;
            }
            ReceiverEvent::Cleanup(id) => {
                self.cleanup_context(&id, false);
            }
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
