use std::{marker::PhantomData, str::FromStr};

use super::StaticallySized;

#[derive(Debug, Copy, Clone)]
pub struct Unevaluated<T> {
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
unevaluated_op!(Sub, sub);

impl<T> From<T> for Unevaluated<T> {
    fn from(_: T) -> Self {
        Self::default()
    }
}

impl<T> FromStr for Unevaluated<T>
where
    T: FromStr,
{
    type Err = <T as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        T::from_str(s)?;
        Ok(Self::default())
    }
}

impl<T> num::Num for Unevaluated<T>
where
    T: num::Num,
{
    type FromStrRadixErr = <T as num::Num>::FromStrRadixErr;

    fn from_str_radix(str: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        T::from_str_radix(str, radix)?;
        Ok(Self::default())
    }
}

impl<T> num::One for Unevaluated<T>
where
    T: num::One,
{
    fn one() -> Self {
        Self::default()
    }
}

impl<T> num::Zero for Unevaluated<T>
where
    T: num::Zero,
{
    fn zero() -> Self {
        Self::default()
    }

    fn is_zero(&self) -> bool {
        panic!("Cannot evaluate is_zero on an unevaluated<T>")
    }
}

impl<T: PartialOrd> PartialOrd for Unevaluated<T> {
    fn partial_cmp(&self, _: &Self) -> Option<std::cmp::Ordering> {
        None
    }
}

impl<T: AddAssign> AddAssign<Unevaluated<T>> for Unevaluated<T> {
    fn add_assign(&mut self, _: Unevaluated<T>) {} // No-op
}
