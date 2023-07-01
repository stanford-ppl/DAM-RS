pub mod tensor;
pub mod unevaluated;

pub trait DAMType: Sync + Send + Copy + Default + core::fmt::Debug + std::cmp::PartialEq {
    // Returns the size of the object in BITS
    fn dam_size(&self) -> usize;
}

pub trait StaticallySized:
    Sync + Send + Copy + Default + core::fmt::Debug + std::cmp::PartialEq
{
    const SIZE: usize;
}

impl<T> DAMType for T
where
    T: StaticallySized,
{
    fn dam_size(&self) -> usize {
        Self::SIZE
    }
}

impl StaticallySized for bool {
    const SIZE: usize = 1;
}

impl StaticallySized for u16 {
    const SIZE: usize = 16;
}

pub trait IndexLike: DAMType + TryInto<usize> + num::Num {
    fn to_usize(self) -> usize {
        match self.try_into() {
            Ok(s) => s,
            Err(_) => panic!("Could not convert {self:?} to usize!"),
        }
    }
}

impl<T> IndexLike for T where T: DAMType + Into<usize> + num::Num {}

pub trait IntegerLike: DAMType + num::Num {}

pub trait Cleanable {
    fn cleanup(&mut self);
}
