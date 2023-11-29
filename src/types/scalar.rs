//! Simple scalar implementations of DAMType
//! This registers the common datatypes as statically sized DAMTypes, such as u64, f32, etc.

use super::StaticallySized;

macro_rules! builtin_ss {
    ($tp: tt, $nbits: literal) => {
        impl StaticallySized for $tp {
            const SIZE: usize = $nbits;
        }
    };
}

builtin_ss!(bool, 1);
builtin_ss!(u8, 8);
builtin_ss!(i8, 8);
builtin_ss!(u16, 16);
builtin_ss!(i16, 16);
builtin_ss!(u32, 32);
builtin_ss!(i32, 32);
builtin_ss!(i64, 64);
builtin_ss!(u64, 64);

builtin_ss!(f32, 32);
builtin_ss!(f64, 64);

impl StaticallySized for usize {
    const SIZE: usize = unimplemented!();
    // usize is a type with a static size, but the value of the size is platform-dependent.
    // Therefore, we implement the trait StaticallySized but keep SIZE unimplemented.
}

impl<A: StaticallySized, B: StaticallySized> StaticallySized for (A, B) {
    const SIZE: usize = A::SIZE + B::SIZE;
}

impl StaticallySized for () {
    const SIZE: usize = 0;
    // This type is used to make it explicit that we don't care about the value.
}

#[cfg(test)]
mod tests {
    use crate::types::{DAMType, StaticallySized};

    #[test]
    fn test_ndarray() {
        let tup_a: (i32, i32) = (5, 5);

        let tup_b: (i32, i64) = (5, 5);

        let i32_size = i32::SIZE;
        let i64_size = i64::SIZE;

        assert!(tup_a.dam_size() == (i32_size + i32_size));
        assert!(tup_b.dam_size() == (i32_size + i64_size));

        dbg!(tup_a);
        dbg!(tup_b);
    }
}
