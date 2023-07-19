use std::sync::{atomic::AtomicUsize, RwLock};

use dam_core::{time::Time, ContextView, TimeView};

use crate::context::Context;

use super::ChannelFlavor;

type ViewType = Option<TimeView>;

#[derive(Default)]
struct ViewData {
    pub sender: ViewType,
    pub receiver: ViewType,
}

pub(crate) struct ViewStruct {
    views: RwLock<ViewData>,
    flavor: ChannelFlavor,

    current_send_receive_delta: AtomicUsize,
}

impl ViewStruct {
    pub fn new(flavor: ChannelFlavor) -> Self {
        Self {
            views: Default::default(),
            flavor,
            current_send_receive_delta: AtomicUsize::new(0),
        }
    }

    pub fn attach_sender(&self, sender: &dyn Context) {
        self.views.write().unwrap().sender = Some(sender.view());
    }

    pub fn attach_receiver(&self, receiver: &dyn Context) {
        self.views.write().unwrap().receiver = Some(receiver.view());
    }

    pub fn register_send(&self) -> usize {
        self.current_send_receive_delta
            .fetch_add(1, std::sync::atomic::Ordering::AcqRel)
    }

    pub fn register_recv(&self) -> usize {
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
}
