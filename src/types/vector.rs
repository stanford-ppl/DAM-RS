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
    use crate::types::{DAMType, StaticallySized};

    #[test]
    fn test_ndarray() {
        let vec_a: Vec<i32> = vec![1, 2, 3];

        let ref_size = i32::SIZE * 3;

        assert!(vec_a.dam_size() == ref_size);
        dbg!(vec_a);
    }
}
