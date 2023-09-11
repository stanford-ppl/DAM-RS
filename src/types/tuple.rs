use super::{DAMType, StaticallySized};

macro_rules! builtin_tup {
    ($tp: tt, $tup: tt, $len: literal) => {
        impl<$tp> DAMType for $tup
        where
            $tp: PartialEq + std::fmt::Debug + Clone + Default + Sync + Send + StaticallySized,
        {
            fn dam_size(&self) -> usize {
                $len * $tp::SIZE
            }
        }
    };
}

builtin_tup!(A, (A, A), 2);
builtin_tup!(A, (A, A, A), 3);
builtin_tup!(A, (A, A, A, A), 4);
builtin_tup!(A, (A, A, A, A, A), 5);
builtin_tup!(A, (A, A, A, A, A, A), 6);

#[cfg(test)]
mod tests {
    use crate::types::{DAMType, StaticallySized};

    #[test]
    fn test_ndarray() {
        let tup_a: (i32, i32) = (5, 5);

        let ref_size = i32::SIZE * 2;

        assert!(tup_a.dam_size() == ref_size);

        dbg!(tup_a);
    }
}
