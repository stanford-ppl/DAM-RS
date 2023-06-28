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

pub trait Cleanable {
    fn cleanup(&mut self);
}
