#[cfg(test)]

mod tests {
    use std::sync::{Arc, Mutex};

    use dam_rs::{
        channel::ChannelElement,
        context::{
            function_context::FunctionContext, parent::BasicParentContext, ParentContext, *,
        },
    };

    use dam_rs::channel::Recv;

    #[test]
    fn simple_channel_test() {
        const TEST_SIZE: i32 = 32;
        let mut writer = FunctionContext::default();
        let mut reader = FunctionContext::default();
        let (mut snd, mut rcv) = dam_rs::channel::Bounded::<i32>::make(8);
        snd.attach_sender(&writer);
        rcv.attach_receiver(&reader);
        let send_mut = Mutex::new(snd);
        let rcv_mut = Mutex::new(rcv);
        writer.set_run(Arc::new(move |wr| {
            let mut sender = send_mut.lock().unwrap();
            for i in 0..TEST_SIZE {
                println!("Trying to send {i}");
                sender
                    .send(ChannelElement::new(wr.time.tick() + 1, i))
                    .unwrap();
                wr.time.incr_cycles(1);
                println!("Sending {}", i);
            }
        }));

        reader.set_run(Arc::new(move |rd| {
            let mut receiver = rcv_mut.lock().unwrap();
            for i in 0..TEST_SIZE {
                loop {
                    let res = receiver.recv();
                    println!("Trying to read {}, Time={:#?}", i, rd.time.tick());
                    match res {
                        Recv::Something(ce) => {
                            rd.time.advance(ce.time);
                            println!("Read {}", ce.data);
                            assert_eq!(ce.data, i);
                            break;
                        }
                        Recv::Nothing(time) => {
                            rd.time.advance(time);
                            rd.time.incr_cycles(1);
                            println!("Recieved nothing, waiting");
                        }
                        Recv::Closed => {
                            panic!("Channel was prematurely closed!");
                        }
                    }
                }
                rd.time.incr_cycles(1);
            }
        }));

        let mut parent = BasicParentContext::default();
        parent.add_child(&mut writer);
        parent.add_child(&mut reader);
        parent.init();
        parent.run();
        parent.cleanup();
    }
}
