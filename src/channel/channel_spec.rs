use crate::shim::Mutex;

use crate::{
    context::Context,
    datastructures::{Identifier, Time},
    view::{ContextView, TimeView},
};

use super::ChannelID;

type ViewType = Option<TimeView>;

/// The basic specification of a connection.
pub(crate) struct ChannelSpec {
    sender_view: Mutex<ViewType>,
    receiver_view: Mutex<ViewType>,

    sender_id: Mutex<Option<Identifier>>,
    receiver_id: Mutex<Option<Identifier>>,
    channel_id: ChannelID,
    capacity: Option<usize>,
    send_latency: u64,
    response_latency: u64,
}

/// An inline version of the specification. This avoids needing an extra Arc/indirection to get back to the original object.
pub(crate) struct InlineSpec {
    pub capacity: Option<usize>,
    pub send_latency: u64,
    pub response_latency: u64,

    sender_view: ViewType,
    receiver_view: ViewType,
}

impl ChannelSpec {
    pub fn new(
        capacity: Option<usize>,
        send_latency: Option<u64>,
        resp_latency: Option<u64>,
    ) -> Self {
        let lat = send_latency.unwrap_or(1);
        let resp_lat = resp_latency.unwrap_or(1);
        assert!(lat > 0);
        assert!(resp_lat > 0);
        Self {
            sender_view: Default::default(),
            receiver_view: Default::default(),
            sender_id: Mutex::new(None),
            receiver_id: Mutex::new(None),
            channel_id: ChannelID::new(),
            capacity,
            send_latency: lat,
            response_latency: resp_lat,
        }
    }

    pub fn sender_id(&self) -> Option<Identifier> {
        *self.sender_id.lock().unwrap()
    }

    // These might be unused if dot graph generation is off.
    #[allow(unused)]
    pub fn latency(&self) -> u64 {
        self.send_latency
    }

    #[allow(unused)]
    pub fn resp_latency(&self) -> u64 {
        self.response_latency
    }

    pub fn receiver_id(&self) -> Option<Identifier> {
        *self.receiver_id.lock().unwrap()
    }

    pub fn attach_sender(&self, sender: &dyn Context) {
        *self.sender_view.lock().unwrap() = Some(sender.view());
        *self.sender_id.lock().unwrap() = Some(sender.id());
    }

    pub fn attach_receiver(&self, receiver: &dyn Context) {
        *self.receiver_view.lock().unwrap() = Some(receiver.view());
        *self.receiver_id.lock().unwrap() = Some(receiver.id());
    }

    pub fn capacity(&self) -> Option<usize> {
        self.capacity
    }

    pub fn id(&self) -> ChannelID {
        self.channel_id
    }

    pub(crate) fn make_inline(&self) -> InlineSpec {
        InlineSpec {
            capacity: self.capacity,
            send_latency: self.send_latency,
            response_latency: self.response_latency,
            sender_view: self.sender_view.lock().unwrap().clone(),
            receiver_view: self.receiver_view.lock().unwrap().clone(),
        }
    }
}

impl InlineSpec {
    pub fn wait_until_sender(&self, time: Time) -> Time {
        self.sender_view.as_ref().unwrap().wait_until(time)
    }

    pub fn sender_tlb(&self) -> Time {
        self.sender_view.as_ref().unwrap().tick_lower_bound()
    }

    pub fn wait_until_receiver(&self, time: Time) -> Time {
        self.receiver_view.as_ref().unwrap().wait_until(time)
    }

    pub fn receiver_tlb(&self) -> Time {
        self.receiver_view.as_ref().unwrap().tick_lower_bound()
    }
}
