pub mod scalar;
pub mod tensor;
pub mod unevaluated;

/// The basic definition of a DAM type.
pub trait DAMType: Sync + Send + Default + core::fmt::Debug + Clone {
    /// Returns the size of the object in BITS
    fn dam_size(&self) -> usize;
}

/// Represents a statically sized DAMType
pub trait StaticallySized {
    /// The size in BITS of an element
    const SIZE: usize;
}

impl<T> DAMType for T
where
    T: StaticallySized + Sync + Send + Default + core::fmt::Debug + Clone,
{
    fn dam_size(&self) -> usize {
        Self::SIZE
    }
}

/// DAMTypes which are Index-Like are ones which can be converted into usize.
/// These are used to index into arrays and memories.
pub trait IndexLike: DAMType + TryInto<usize> + num::Num {
    /// Converts the object into a usize.
    fn to_usize(&self) -> usize {
        match self.clone().try_into() {
            Ok(s) => s,
            Err(_) => panic!("Could not convert {self:?} to usize!"),
        }
    }
}

impl<T> IndexLike for T where T: DAMType + Into<usize> + num::Num {}

/// DAM types which behave like numbers, used for trait bounds in contexts.
pub trait IntegerLike: DAMType + num::Num {}

/// Generic trait for objects which may be cleaned up early
pub trait Cleanable {
    /// An early cleanup trigger, for freeing resources, managing state, etc.
    fn cleanup(&mut self);
}
