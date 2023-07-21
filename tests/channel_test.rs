#[cfg(test)]
mod tests {
    use dam_core::{ContextView, TimeViewable};
    use dam_rs::{
        channel::{bounded_with_flavor, ChannelFlavor},
        context::{
            function_context::FunctionContext, generator_context::GeneratorContext,
            parent::BasicParentContext, Context,
        },
    };

    fn test_channel(flavor: ChannelFlavor) {
        let test_size = 5;
        let mut parent = BasicParentContext::default();
        let (snd, mut rcv) = bounded_with_flavor(2, flavor);
        let mut sender = GeneratorContext::new(move || (0..test_size).into_iter(), snd);
        let mut recv_ctx = FunctionContext::new();
        rcv.attach_receiver(&recv_ctx);
        recv_ctx.set_run(move |time| {
            for iter in 0..test_size {
                // for _ in 0..test_size {
                //     rcv.peek_next(time);
                // }
                // rcv.peek_next(time);
                time.incr_cycles(u64::try_from(iter).unwrap());
                let res = match rcv.dequeue(time) {
                    dam_rs::channel::Recv::Something(x) => x.data,
                    _ => panic!("This shouldn't happen!"),
                };
                assert_eq!(res, iter);
            }
        });
        parent.add_child(&mut sender);
        parent.add_child(&mut recv_ctx);
        parent.init();
        parent.run();
        parent.cleanup();
        println!(
            "Flavor: {flavor:?}, ticks: {:?}",
            recv_ctx.view().tick_lower_bound()
        );
    }

    #[test]
    fn test_channel_cyclic() {
        test_channel(ChannelFlavor::Cyclic);
    }

    #[test]
    fn test_channel_acyclic() {
        test_channel(ChannelFlavor::Acyclic);
    }
}
