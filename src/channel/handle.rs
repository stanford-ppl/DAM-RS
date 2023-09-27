// This provides an inner-mutability based way of modifying a channel.
// The key feature we need here is to be able to set up a graph (i.e. pass ownership around)
// And later swap the underlying implementation of the sender/receivers.

use std::sync::Arc;

use crossbeam::channel;
use dam_core::{identifier::Identifier, sync_unsafe::SyncUnsafeCell, time::Time};

use super::{
    channel_spec::ChannelSpec,
    receiver::{undefined::UndefinedReceiver, *},
    sender::{
        AcyclicSender, CyclicSender, InfiniteSender, SenderImpl, UndefinedSender, VoidSender,
    },
    ChannelElement, ChannelFlavor, ChannelID,
};

pub(crate) trait ChannelHandle {
    fn set_flavor(&self, flavor: ChannelFlavor);
    fn sender(&self) -> Option<Identifier>;
    fn receiver(&self) -> Option<Identifier>;
    fn id(&self) -> ChannelID;
    fn spec(&self) -> Arc<ChannelSpec>;
}

pub(crate) struct ChannelData<T: Clone> {
    sender: SyncUnsafeCell<SenderImpl<T>>,
    receiver: SyncUnsafeCell<ReceiverImpl<T>>,
    channel_spec: Arc<ChannelSpec>,
}

impl<T: Clone> ChannelData<T> {
    pub fn new(spec: Arc<ChannelSpec>) -> Self {
        Self {
            sender: SyncUnsafeCell::new(UndefinedSender::new(spec.clone()).into()),
            receiver: SyncUnsafeCell::new(UndefinedReceiver::new(spec.clone()).into()),
            channel_spec: spec,
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub(super) fn sender(&self) -> &mut SenderImpl<T> {
        unsafe { self.sender.get().as_mut().unwrap() }
    }

    #[allow(clippy::mut_from_ref)]
    pub(super) fn receiver(&self) -> &mut ReceiverImpl<T> {
        unsafe { self.receiver.get().as_mut().unwrap() }
    }
}

impl<T: Clone> ChannelHandle for ChannelData<T> {
    fn set_flavor(&self, flavor: ChannelFlavor) {
        let make_channel_data = |underlying| ReceiverData::<T> {
            spec: self.channel_spec.make_inline(),
            underlying,
            head: None,
        };
        match self.channel_spec.capacity() {
            Some(capacity) => {
                let (tx, rx) = channel::bounded::<ChannelElement<T>>(capacity);
                let (resp_t, resp_r) = channel::bounded::<Time>(capacity);
                match flavor {
                    ChannelFlavor::Unknown => panic!("Cannot set flavor to unknown!"),
                    ChannelFlavor::Acyclic => {
                        *self.sender() =
                            AcyclicSender::new(tx, resp_r, self.channel_spec.make_inline()).into();
                        *self.receiver() = BoundedAcyclicReceiver {
                            data: make_channel_data(rx),
                            resp: resp_t,
                        }
                        .into();
                    }
                    ChannelFlavor::Cyclic => {
                        *self.sender() =
                            CyclicSender::new(tx, resp_r, self.channel_spec.make_inline()).into();
                        *self.receiver() = BoundedCyclicReceiver {
                            data: make_channel_data(rx),
                            resp: resp_t,
                        }
                        .into();
                    }
                    ChannelFlavor::Void => *self.sender() = VoidSender::default().into(),
                }
            }

            // Unbounded channel
            None => {
                //
                match flavor {
                    ChannelFlavor::Unknown => panic!("Cannot set flavor to unknown!"),
                    ChannelFlavor::Acyclic => {
                        let (snd, rcv) = channel::unbounded();

                        *self.sender() = InfiniteSender::new(
                            super::sender::SenderState::Open(snd),
                            self.channel_spec.make_inline(),
                        )
                        .into();

                        *self.receiver() = InfiniteAcyclicReceiver {
                            data: make_channel_data(rcv),
                        }
                        .into();
                    }
                    ChannelFlavor::Cyclic => {
                        let (snd, rcv) = channel::unbounded();

                        *self.sender() = InfiniteSender::new(
                            super::sender::SenderState::Open(snd),
                            self.channel_spec.make_inline(),
                        )
                        .into();

                        *self.receiver() = InfiniteCyclicReceiver {
                            data: make_channel_data(rcv),
                        }
                        .into();
                    }
                    ChannelFlavor::Void => *self.sender() = VoidSender::default().into(),
                }
            }
        }
    }

    fn sender(&self) -> Option<Identifier> {
        self.channel_spec.sender_id()
    }

    fn receiver(&self) -> Option<Identifier> {
        self.channel_spec.receiver_id()
    }

    fn id(&self) -> ChannelID {
        self.channel_spec.id()
    }

    fn spec(&self) -> Arc<ChannelSpec> {
        self.channel_spec.clone()
    }
}
