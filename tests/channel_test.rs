#[cfg(test)]
mod tests {

    use dam::{channel::ChannelElement, simulation::*, utility_contexts::FunctionContext};

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
            let mut rng = fastrand::Rng::new();
            for iter in 0..test_size {
                // sleep for some random amount of time
                dam::shim::sleep(std::time::Duration::from_millis(rng.u64(0..=MAX_MS_SLEEP)));
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
            let mut rng = fastrand::Rng::new();
            for iter in 0..test_size {
                dam::shim::sleep(std::time::Duration::from_millis(rng.u64(0..=100)));
                match rcv.dequeue(time) {
                    Ok(ChannelElement { time: _, data }) => {
                        assert_eq!(data, iter);
                    }
                    Err(_) => {
                        panic!("Premature termination of channel")
                    }
                }
                time.incr_cycles(1);
            }
        });
        ctx.add_child(receiver);

        #[allow(unused)]
        let summary = ctx
            .initialize(
                InitializationOptionsBuilder::default()
                    .run_flavor_inference(flavor_inference)
                    .build()
                    .unwrap(),
            )
            .unwrap()
            .run(RunOptions::default());

        #[cfg(feature = "dot")]
        {
            println!("{}", summary.to_dot_string());
        }
    }
}
