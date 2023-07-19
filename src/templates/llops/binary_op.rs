use crate::{
    channel::{Receiver, Sender},
    context::Context,
    templates::{
        ops::ALUOp,
        pcu::{PCUConfig, PipelineStage, PCU},
    },
    types::DAMType,
};
use ndarray::{ArrayBase, Dim, OwnedRepr};

pub fn test_add<A>(
    arg1: ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>,
    arg2: ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>,
) -> ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>
where
    ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>:
        std::ops::Add<Output = ArrayBase<OwnedRepr<A>, Dim<[usize; 2]>>>,
{
    arg1 + arg2
}

pub fn make_pcu<A: Clone>(
    arg1: Receiver<ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>>,
    arg2: Receiver<ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>>,
    res: Sender<ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>>,
    op: ALUOp<ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>>,
) -> impl Context
where
    ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>: DAMType,
{
    let ingress_op = PCU::<ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>>::READ_ALL_INPUTS;
    let egress_op = PCU::<ArrayBase<OwnedRepr<A>, Dim<[usize; 1]>>>::WRITE_ALL_RESULTS;

    let mut pcu = PCU::new(
        PCUConfig {
            pipeline_depth: 1,
            num_registers: 2,
        },
        ingress_op,
        egress_op,
    );

    pcu.push_stage(PipelineStage {
        op,
        forward: vec![],
        prev_register_ids: vec![0, 1],
        next_register_ids: vec![],
        output_register_ids: vec![0],
    });
    pcu.add_input_channel(arg1);
    pcu.add_input_channel(arg2);
    pcu.add_output_channel(res);

    pcu
}

#[cfg(test)]
mod tests {
    use crate::{
        channel::unbounded,
        context::{
            checker_context::CheckerContext, generator_context::GeneratorContext,
            parent::BasicParentContext, Context,
        },
        templates::ops::ALUMulOp,
    };

    use super::make_pcu;
    use super::test_add;
    use ndarray::{array, ArrayBase, Dim, OwnedRepr};

    #[test]
    fn add_test() {
        let a = array![[1, 2, 3], [3, 4, 5]];
        let b = array![[5, 6, 2], [7, 8, 1]];
        let c = array![[6, 8, 5], [10, 12, 6]];
        assert_eq!(test_add(a, b), c);
    }

    #[test]
    fn binary_pcu_test() {
        /*
           gen1 - |arg1_send ... arg1_recv| - pcu - |pcu_out_send ... pcu_out_recv|- checker
           gen1 - |arg2_send ... arg2_recv| /
        */
        let (arg1_send, arg1_recv) = unbounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 1]>>>();
        let (arg2_send, arg2_recv) = unbounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 1]>>>();
        let (pcu_out_send, pcu_out_recv) =
            unbounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 1]>>>();
        let mut gen1 = GeneratorContext::new(
            || (0i32..10).map(|x| array![x, x, x, x, x, x, x, x, x, x, x, x, x, x, x, x]),
            arg1_send,
        );
        let mut gen2 = GeneratorContext::new(
            || (0i32..10).map(|x| array![x, x, x, x, x, x, x, x, x, x, x, x, x, x, x, x]),
            arg2_send,
        );
        let mut binary_pcu = make_pcu(arg1_recv, arg2_recv, pcu_out_send, ALUMulOp());
        let mut checker = CheckerContext::new(
            || {
                (0i32..10).map(|x| {
                    array![
                        x * x,
                        x * x,
                        x * x,
                        x * x,
                        x * x,
                        x * x,
                        x * x,
                        x * x,
                        x * x,
                        x * x,
                        x * x,
                        x * x,
                        x * x,
                        x * x,
                        x * x,
                        x * x
                    ]
                })
            },
            pcu_out_recv,
        );
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut gen2);
        parent.add_child(&mut binary_pcu);
        parent.add_child(&mut checker);
        parent.init();
        parent.run();
        parent.cleanup();
    }
}
