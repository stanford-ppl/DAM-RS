use std::sync::{atomic::AtomicUsize, Mutex, RwLock};

use dam_core::{identifier::Identifier, time::Time, ContextView, TimeView};

use crate::context::Context;

use super::ChannelID;

type ViewType = Option<TimeView>;

#[derive(Default)]
struct ViewData {
    pub sender: ViewType,
    pub receiver: ViewType,
}

pub struct ChannelSpec {
    views: RwLock<ViewData>,

    current_send_receive_delta: AtomicUsize,
    total_sent: AtomicUsize,
    total_received: AtomicUsize,

    sender_id: Mutex<Option<Identifier>>,
    receiver_id: Mutex<Option<Identifier>>,
    channel_id: ChannelID,
    capacity: Option<usize>,
}

impl ChannelSpec {
    pub fn new(capacity: Option<usize>) -> Self {
        Self {
            views: Default::default(),
            current_send_receive_delta: AtomicUsize::new(0),
            total_sent: AtomicUsize::new(0),
            total_received: AtomicUsize::new(0),
            sender_id: Mutex::new(None),
            receiver_id: Mutex::new(None),
            channel_id: ChannelID::new(),
            capacity,
        }
    }

    pub fn sender_id(&self) -> Option<Identifier> {
        self.sender_id.lock().unwrap().clone()
    }

    pub fn receiver_id(&self) -> Option<Identifier> {
        self.receiver_id.lock().unwrap().clone()
    }

    pub fn attach_sender(&self, sender: &dyn Context) {
        self.views.write().unwrap().sender = Some(sender.view());
    }

    pub fn attach_receiver(&self, receiver: &dyn Context) {
        self.views.write().unwrap().receiver = Some(receiver.view());
    }

    pub fn register_send(&self) -> usize {
        self.total_sent
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.current_send_receive_delta
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel)
    }

    pub fn register_recv(&self) -> usize {
        self.total_received
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.current_send_receive_delta
            .fetch_sub(1, std::sync::atomic::Ordering::AcqRel)
    }

    pub fn sender_tlb(&self) -> Time {
        self.views
            .read()
            .unwrap()
            .sender
            .as_ref()
            .unwrap()
            .tick_lower_bound()
    }

    pub fn receiver_tlb(&self) -> Time {
        self.views
            .read()
            .unwrap()
            .receiver
            .as_ref()
            .unwrap()
            .tick_lower_bound()
    }

    pub fn current_srd(&self) -> usize {
        self.current_send_receive_delta
            .load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn wait_until_sender(&self, time: Time) -> Time {
        self.views
            .read()
            .unwrap()
            .sender
            .as_ref()
            .unwrap()
            .wait_until(time)
    }

    pub fn wait_until_receiver(&self, time: Time) -> Time {
        self.views
            .read()
            .unwrap()
            .receiver
            .as_ref()
            .unwrap()
            .wait_until(time)
    }

    pub fn capacity(&self) -> Option<usize> {
        self.capacity
    }
}
