use super::{DAMType, StaticallySized};

impl<A> DAMType for Vec<A>
where
    A: Sync + Send + Default + core::fmt::Debug + std::cmp::PartialEq + Clone + StaticallySized,
{
    fn dam_size(&self) -> usize {
        self.len() * A::SIZE
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Mul;

    use ndarray::array;

    use crate::types::{DAMType, StaticallySized};

    #[test]
    fn test_ndarray() {
        let mat_a = array![[1, 2], [3, 4]];
        let mat_b = array![[5, 6], [7, 8]];

        let ref_size = i32::SIZE * 4;

        assert!(mat_a.dam_size() == ref_size);
        assert!(mat_b.dam_size() == ref_size);

        let mat_c = (&mat_a).mul(&mat_b);
        dbg!(mat_c);

        let mat_d = mat_a + mat_b;

        dbg!(mat_d);
    }
}
