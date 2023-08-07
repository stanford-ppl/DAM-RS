use std::sync::{atomic::AtomicUsize, Mutex};

use dam_core::{
    identifier::Identifier, sync_unsafe::SyncUnsafeCell, time::Time, ContextView, TimeView,
};

use crate::context::Context;

use super::ChannelID;

type ViewType = Option<TimeView>;

#[derive(Default)]
struct ViewData {
    pub sender: ViewType,
    pub receiver: ViewType,
}

pub struct ChannelSpec {
    sender_view: SyncUnsafeCell<ViewType>,
    receiver_view: SyncUnsafeCell<ViewType>,

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
            sender_view: Default::default(),
            receiver_view: Default::default(),
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
        *self.sender_id.lock().unwrap()
    }

    pub fn receiver_id(&self) -> Option<Identifier> {
        *self.receiver_id.lock().unwrap()
    }

    pub fn attach_sender(&self, sender: &dyn Context) {
        unsafe {
            *self.sender_view.get() = Some(sender.view());
        }
        *self.sender_id.lock().unwrap() = Some(sender.id());
    }

    pub fn attach_receiver(&self, receiver: &dyn Context) {
        unsafe {
            *self.receiver_view.get() = Some(receiver.view());
        }
        *self.receiver_id.lock().unwrap() = Some(receiver.id());
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
        unsafe {
            self.sender_view
                .get()
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .tick_lower_bound()
        }
    }

    pub fn receiver_tlb(&self) -> Time {
        unsafe {
            self.receiver_view
                .get()
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .tick_lower_bound()
        }
    }

    pub fn current_srd(&self) -> usize {
        self.current_send_receive_delta
            .load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn wait_until_sender(&self, time: Time) -> Time {
        unsafe {
            self.sender_view
                .get()
                .as_mut()
                .unwrap()
                .as_ref()
                .unwrap()
                .wait_until(time)
        }
    }

    pub fn wait_until_receiver(&self, time: Time) -> Time {
        unsafe {
            self.receiver_view
                .get()
                .as_mut()
                .unwrap()
                .as_ref()
                .unwrap()
                .wait_until(time)
        }
    }

    pub fn capacity(&self) -> Option<usize> {
        self.capacity
    }

    pub fn id(&self) -> ChannelID {
        self.channel_id
    }
}
