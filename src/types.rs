pub trait DAMType: Sync + Send + Copy + Default + core::fmt::Debug + std::cmp::PartialEq {
    // Returns the size of the object in BITS
    fn dam_size() -> usize;
}

impl DAMType for bool {
    fn dam_size() -> usize {
        1
    }
}

impl DAMType for i32 {
    fn dam_size() -> usize {
        32
    }
}

impl DAMType for u16 {
    fn dam_size() -> usize {
        16
    }
}

impl DAMType for u32 {
    fn dam_size() -> usize {
        32
    }
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
