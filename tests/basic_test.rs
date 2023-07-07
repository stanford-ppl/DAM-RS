#[cfg(test)]

mod tests {

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
        let mut writer = FunctionContext::new();
        let mut reader = FunctionContext::new();
        let (mut snd, mut rcv) = dam_rs::channel::bounded::<i32>(8);
        snd.attach_sender(&writer);
        rcv.attach_receiver(&reader);
        writer.set_run(move |time| {
            for i in 0..TEST_SIZE {
                println!("Trying to send {i}");
                snd.send(ChannelElement::new(time.tick() + 1, i)).unwrap();
                time.incr_cycles(1);
                println!("Sending {}", i);
            }
        });

        reader.set_run(move |time| {
            for i in 0..TEST_SIZE {
                loop {
                    let res = rcv.recv();
                    println!("Trying to read {}, Time={:#?}", i, time.tick());
                    match res {
                        Recv::Something(ce) => {
                            time.advance(ce.time);
                            println!("Read {}", ce.data);
                            assert_eq!(ce.data, i);
                            break;
                        }
                        Recv::Nothing(new_time) => {
                            time.advance(new_time);
                            time.incr_cycles(1);
                            println!("Recieved nothing, waiting");
                        }
                        Recv::Closed => {
                            panic!("Channel was prematurely closed!");
                        }
                        Recv::Unknown => {
                            unreachable!();
                        }
                    }
                }
                time.incr_cycles(1);
            }
        });

        let mut parent = BasicParentContext::default();
        parent.add_child(&mut writer);
        parent.add_child(&mut reader);
        parent.init();
        parent.run();
        parent.cleanup();
    }
}
