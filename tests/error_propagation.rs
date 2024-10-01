use dam::context_tools::*;
use dam::simulation::*;
use dam::utility_contexts::*;

fn splice<'a, T: Clone + DAMType + 'a>(
    pb: &mut ProgramBuilder<'a>,
    input: Receiver<T>,
) -> (Receiver<T>, Receiver<T>) {
    let (snd1, rcv1) = pb.unbounded();
    let (snd2, rcv2) = pb.unbounded();

    let mut bc = BroadcastContext::new(input);
    bc.add_target(snd1);
    bc.add_target(snd2);
    pb.add_child(bc);
    (rcv1, rcv2)
}

/// Simple test to check whether abort catching works
#[test]
fn splice_test() {
    let mut parent = ProgramBuilder::default();
    let (snd, rcv) = parent.unbounded();
    let sender = GeneratorContext::new(|| 0..20, snd);

    let (rcv1, rcv2) = splice(&mut parent, rcv);

    let printer = PrinterContext::new(rcv1);
    let checker = CheckerContext::new(|| 10..30, rcv2);

    parent.add_child(sender);
    parent.add_child(checker);
    parent.add_child(printer);

    let executed = parent
        .initialize(InitializationOptionsBuilder::default().build().unwrap())
        .unwrap()
        .run(RunOptions::default());
    executed.dump_failures();
    assert!(!executed.passed());
}
