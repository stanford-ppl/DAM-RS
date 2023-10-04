#[cfg(test)]
mod tests {

    use dam_rs::{
        channel::ChannelElement, context::function_context::FunctionContext, simulation::*,
    };
    use rand::Rng;

    // The tests will take TEST_SIZE * MAX_MS_SLEEP / 2 on average.
    const TEST_SIZE: i32 = 1 << 8;
    const MAX_MS_SLEEP: u64 = 100;

    #[test]
    fn test_channel_bounded_noinfer() {
        run_channel_test(TEST_SIZE, false, Some(16));
    }

    #[test]
    fn test_channel_bounded_infer() {
        run_channel_test(TEST_SIZE, true, Some(16));
    }

    #[test]
    fn test_channel_unbounded_noinfer() {
        run_channel_test(TEST_SIZE, false, None);
    }

    #[test]
    fn test_channel_unbounded_infer() {
        run_channel_test(TEST_SIZE, true, None);
    }

    fn run_channel_test(test_size: i32, flavor_inference: bool, capacity: Option<usize>) {
        let mut ctx = ProgramBuilder::default();

        let (snd, rcv) = match capacity {
            Some(cap) => ctx.bounded(cap),
            None => ctx.unbounded(),
        };

        let mut sender = FunctionContext::default();
        snd.attach_sender(&sender);
        sender.set_run(move |time| {
            let mut rng = rand::thread_rng();
            for iter in 0..test_size {
                // sleep for some random amount of time
                std::thread::sleep(std::time::Duration::from_millis(
                    rng.gen_range(0..=MAX_MS_SLEEP),
                ));
                let cur_time = time.tick();
                snd.enqueue(time, ChannelElement::new(cur_time + (iter as u64), iter))
                    .unwrap();

                time.incr_cycles(1);
            }
        });
        ctx.add_child(sender);

        let mut receiver = FunctionContext::default();
        rcv.attach_receiver(&receiver);
        receiver.set_run(move |time| {
            let mut rng = rand::thread_rng();
            for iter in 0..test_size {
                std::thread::sleep(std::time::Duration::from_millis(rng.gen_range(0..=100)));
                match rcv.dequeue(time) {
                    dam_rs::channel::DequeueResult::Something(ChannelElement { time: _, data }) => {
                        assert_eq!(data, iter);
                    }
                    dam_rs::channel::DequeueResult::Closed => {
                        panic!("Premature termination of channel")
                    }
                }
                time.incr_cycles(1);
            }
        });
        ctx.add_child(receiver);
        ctx.initialize(InitializationOptions {
            run_flavor_inference: flavor_inference,
        })
        .unwrap()
        .run(RunMode::Simple);
    }
}
