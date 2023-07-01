use std::marker::PhantomData;

use super::StaticallySized;

#[derive(Debug, Copy, Clone)]
struct Unevaluated<T> {
    _phantom: PhantomData<T>,
}

impl<T: StaticallySized> StaticallySized for Unevaluated<T> {
    const SIZE: usize = T::SIZE;
}

impl<T> Default for Unevaluated<T> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}

impl<T> PartialEq for Unevaluated<T> {
    fn eq(&self, _: &Self) -> bool {
        panic!("Cannot compare unevaluated types!")
    }
}

use std::ops::*;
macro_rules! unevaluated_op {
    ($op_trait: ident, $fname: ident) => {
        impl<T> $op_trait<Unevaluated<T>> for Unevaluated<T>
        where
            T: $op_trait<T>,
        {
            type Output = Unevaluated<T>;

            fn $fname(self, _: Unevaluated<T>) -> Self::Output {
                Unevaluated::<T>::default()
            }
        }

        impl<T> $op_trait<T> for Unevaluated<T>
        where
            T: $op_trait<T>,
        {
            type Output = Unevaluated<T>;

            fn $fname(self, _: T) -> Self::Output {
                Unevaluated::<T>::default()
            }
        }
    };
}

unevaluated_op!(Add, add);
unevaluated_op!(Mul, mul);
unevaluated_op!(Div, div);
unevaluated_op!(Rem, rem);
