use ndarray::{ArrayBase, Dim, OwnedRepr};

pub fn test_add(
    arg1: ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>,
    arg2: ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>>,
) -> ArrayBase<OwnedRepr<i32>, Dim<[usize; 2]>> {
    arg1 + arg2
}

#[cfg(test)]
mod tests {
    use crate::{
        channel::unbounded,
        context::{
            checker_context::CheckerContext, generator_context::GeneratorContext,
            parent::BasicParentContext, Context,
        },
    };

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
    fn generator_checker_test() {
        /*
           gen1 |arg1_send ... arg1_recv| checker
        */
        let (arg1_send, arg1_recv) = unbounded::<ArrayBase<OwnedRepr<i32>, Dim<[usize; 1]>>>();
        let mut gen1 = GeneratorContext::new(|| (0i32..10).map(|x| array![x, x, x]), arg1_send);
        let mut checker = CheckerContext::new(|| (0i32..10).map(|x| array![x, x, x]), arg1_recv);
        let mut parent = BasicParentContext::default();
        parent.add_child(&mut gen1);
        parent.add_child(&mut checker);
        parent.init();
        parent.run();
        parent.cleanup();
    }
}
