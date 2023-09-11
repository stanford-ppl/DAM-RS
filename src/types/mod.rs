pub mod scalar;
pub mod tensor;
pub mod tuple;
pub mod unevaluated;

//Sync + Send + Default + core::fmt::Debug + std::cmp::PartialEq + Clone;
pub trait DAMType: Sync + Send + Default + core::fmt::Debug + std::cmp::PartialEq + Clone {
    // Returns the size of the object in BITS
    fn dam_size(&self) -> usize;
}

pub trait StaticallySized:
    Sync + Send + Default + core::fmt::Debug + std::cmp::PartialEq + Clone
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

pub trait IndexLike: DAMType + TryInto<usize> + num::Num {
    fn to_usize(&self) -> usize {
        match self.clone().try_into() {
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
