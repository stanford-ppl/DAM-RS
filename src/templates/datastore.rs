use parking_lot::RwLock;

use crate::{time::Time, types::DAMType};

#[derive(Clone, Copy, Debug)]
struct StoreElement<T> {
    data: T,
    time: Time,
}

#[derive(Debug, Clone, Copy)]
pub struct Behavior {
    pub mod_address: bool,
    pub use_default_value: bool,
}

#[derive(Debug)]
pub struct Datastore<T> {
    capacity: usize,
    behavior: Behavior,
    underlying: Vec<RwLock<Vec<StoreElement<T>>>>,
}

impl<T: DAMType> Datastore<T> {
    pub fn new(capacity: usize, behavior: Behavior) -> Datastore<T> {
        let mut ds = Datastore {
            capacity,
            behavior,
            underlying: Vec::new(),
        };
        ds.underlying
            .resize_with(capacity, || RwLock::new(Vec::new()));
        ds
    }

    fn safe_addr(&self, addr: usize) -> usize {
        if addr >= self.capacity {
            if self.behavior.mod_address {
                addr % self.capacity
            } else {
                panic!("Out of bounds read at address {addr}");
            }
        } else {
            addr
        }
    }

    pub fn write(&self, addr: usize, data: T, time: Time) {
        let history = self.underlying.get(self.safe_addr(addr)).unwrap();
        let mut hist = history.write();
        let entry = StoreElement { time, data };
        match hist.last() {
            Some(last) if last.time >= time => {
                panic!(
                    "Attempting to write a new element in the past! ({:?} >= {:?})",
                    last.time, time
                );
            }
            _ => hist.push(entry),
        }
    }

    pub fn read(&self, addr: usize, time: Time) -> T {
        let history = self.underlying.get(self.safe_addr(addr)).unwrap();
        let reader = history.read();
        // next_pos holds the location of the first element after the read time.
        let next_pos = reader.iter().position(|elem| elem.time > time);
        match next_pos {
            Some(0) | None => match reader.last() {
                None => {
                    if self.behavior.use_default_value {
                        T::default()
                    } else {
                        panic!("Attempting to read a value before anything was written!")
                    }
                }
                Some(last_val) => last_val.data.clone(),
            },

            Some(x) => reader.get(x - 1).unwrap().data.clone(),
        }
    }
}
