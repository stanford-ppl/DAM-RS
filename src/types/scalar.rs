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

builtin_ss!(usize, 64);

builtin_ss!(f32, 32);
builtin_ss!(f64, 64);
