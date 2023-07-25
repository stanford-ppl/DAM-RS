// This provides an inner-mutability based way of modifying a channel.
// The key feature we need here is to be able to set up a graph (i.e. pass ownership around)
// And later swap the underlying implementation of the sender/receivers.

use std::sync::{Arc, Mutex};

use crossbeam::channel;
use dam_core::{identifier::Identifier, time::Time};

use super::{
    channel_spec::ChannelSpec,
    receiver::{
        AcyclicInfiniteReceiver, AcyclicReceiver, CyclicInfiniteReceiver, CyclicReceiver,
        ReceiverImpl, UndefinedReceiver,
    },
    sender::{
        AcyclicSender, CyclicSender, InfiniteSender, SenderImpl, UndefinedSender, VoidSender,
    },
    ChannelElement, ChannelFlavor,
};

pub(crate) trait ChannelHandle {
    fn set_flavor(&self, flavor: ChannelFlavor);
    fn sender(&self) -> Option<Identifier>;
    fn receiver(&self) -> Option<Identifier>;
}

pub(crate) struct ChannelData<T: Clone> {
    pub(crate) sender: Mutex<SenderImpl<T>>,
    pub(crate) receiver: Mutex<ReceiverImpl<T>>,
    pub(crate) view_struct: Arc<ChannelSpec>,
}

impl<T: Clone> ChannelData<T> {
    pub fn new(spec: Arc<ChannelSpec>) -> Self {
        Self {
            sender: Mutex::new(UndefinedSender::new(spec.clone()).into()),
            receiver: Mutex::new(UndefinedReceiver::new(spec.clone()).into()),
            view_struct: spec,
        }
    }
}

impl<T: Clone> ChannelHandle for ChannelData<T> {
    fn set_flavor(&self, flavor: ChannelFlavor) {
        match self.view_struct.capacity() {
            Some(capacity) => {
                let (tx, rx) = channel::bounded::<ChannelElement<T>>(capacity);
                let (resp_t, resp_r) = channel::bounded::<Time>(capacity);
                match flavor {
                    ChannelFlavor::Unknown => panic!("Cannot set flavor to unknown!"),
                    ChannelFlavor::Acyclic => {
                        *self.sender.lock().unwrap() =
                            AcyclicSender::new(tx, resp_r, capacity, self.view_struct.clone())
                                .into();
                        *self.receiver.lock().unwrap() =
                            AcyclicReceiver::new(rx, resp_t, self.view_struct.clone()).into();
                    }
                    ChannelFlavor::Cyclic => {
                        *self.sender.lock().unwrap() =
                            CyclicSender::new(tx, resp_r, capacity, self.view_struct.clone())
                                .into();
                        *self.receiver.lock().unwrap() =
                            CyclicReceiver::new(rx, resp_t, self.view_struct.clone()).into();
                    }
                    ChannelFlavor::Void => {
                        *self.sender.lock().unwrap() = VoidSender::default().into()
                    }
                }
            }

            // Unbounded channel
            None => {
                //
                match flavor {
                    ChannelFlavor::Unknown => panic!("Cannot set flavor to unknown!"),
                    ChannelFlavor::Acyclic => {
                        let (snd, rcv) = channel::unbounded();

                        *self.sender.lock().unwrap() = InfiniteSender::new(
                            super::sender::SenderState::Open(snd),
                            self.view_struct.clone(),
                        )
                        .into();

                        *self.receiver.lock().unwrap() = AcyclicInfiniteReceiver::new(
                            super::receiver::ReceiverState::Open(rcv),
                            self.view_struct.clone(),
                        )
                        .into();
                    }
                    ChannelFlavor::Cyclic => {
                        let (snd, rcv) = channel::unbounded();

                        *self.sender.lock().unwrap() = InfiniteSender::new(
                            super::sender::SenderState::Open(snd),
                            self.view_struct.clone(),
                        )
                        .into();

                        *self.receiver.lock().unwrap() = CyclicInfiniteReceiver::new(
                            super::receiver::ReceiverState::Open(rcv),
                            self.view_struct.clone(),
                        )
                        .into();
                    }
                    ChannelFlavor::Void => {
                        *self.sender.lock().unwrap() = VoidSender::default().into()
                    }
                }
            }
        }
    }

    fn sender(&self) -> Option<Identifier> {
        self.view_struct.sender_id()
    }

    fn receiver(&self) -> Option<Identifier> {
        self.view_struct.receiver_id()
    }
}
