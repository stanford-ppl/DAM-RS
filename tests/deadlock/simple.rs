#[cfg(test)]
mod tests {

    use dam_core::TimeManager;
    use dam_rs::{
        channel::{ChannelElement, Receiver, Sender},
        context::{function_context::FunctionContext, generator_context::GeneratorContext},
        simulation::Program,
    };

    fn ping_pong(
        time: &mut TimeManager,
        mut snd: Sender<i32>,
        mut rcv: Receiver<i32>,
        test_size: i32,
        num_packets: i32,
    ) {
        for i in 0..test_size {
            for _ in 0..num_packets {
                let packet = ChannelElement::new(time.tick(), i);
                snd.enqueue(time, packet).unwrap();
            }

            for _ in 0..num_packets {
                let res = match rcv.dequeue(time) {
                    dam_rs::channel::Recv::Something(x) => x.data,
                    _ => panic!("Did not receive a proper packet"),
                };
                assert_eq!(res, i);
            }

            time.incr_cycles(1);
        }
    }

    #[test]
    fn test_simple_cyclic_deadlock() {
        let test_size = 5;
        let num_packets = 2;
        let channel_depth = 1;

        let mut parent = Program::default();

        let mut a = FunctionContext::new();
        let mut b = FunctionContext::new();

        let (mut ab_snd, mut ab_rcv) = parent.bounded(channel_depth);
        ab_snd.attach_sender(&a);
        ab_rcv.attach_receiver(&b);

        let (mut ba_snd, mut ba_rcv) = parent.bounded(channel_depth);
        ba_snd.attach_sender(&b);
        ba_rcv.attach_receiver(&a);

        a.set_run(move |time| {
            ping_pong(time, ab_snd, ba_rcv, test_size, num_packets);
        });

        b.set_run(move |time| {
            ping_pong(time, ba_snd, ab_rcv, test_size, num_packets);
        });

        parent.add_child(a);
        parent.add_child(b);
        parent.init();
        parent.run();
        parent.print_graph();
    }
}
