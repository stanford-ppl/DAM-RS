//! Datastores are not real contexts, but can be used to compose more advanced constructs.

use crate::shim::RwLock;

use crate::{datastructures::Time, types::DAMType};
#[derive(Clone, Copy, Debug)]
struct StoreElement<T> {
    data: T,
    time: Time,
}

/// Options for specifying the behavior of a datastore
#[derive(Debug, Clone, Copy)]
pub struct Behavior {
    /// Should the address wrap around on overflow
    pub mod_address: bool,

    /// Should the datastore use the default value for datatypes
    pub use_default_value: bool,
}

/// A Datastore is a time-travelling construct which allows the write side to operate arbitrarily far into the future w.r.t. the read side.
/// It achieves this by recording a history of all writes to the datastore, thereby allowing readers to read arbitrarily far in the past.
#[derive(Debug)]
pub struct Datastore<T> {
    capacity: usize,
    behavior: Behavior,
    underlying: Vec<RwLock<Vec<StoreElement<T>>>>,
}

impl<T: DAMType> Datastore<T> {
    /// Constructs a datastore with a fixed capacity and a predefined behavior.
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

    /// Writes to the datastore at a particular address. The write time of each entry must be monotonically increasing.
    pub fn write(&self, addr: usize, data: T, time: Time) {
        let history = self.underlying.get(self.safe_addr(addr)).unwrap();
        let mut hist = history.write().unwrap();
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

    /// Reads from the datastore. In order to avoid reading a stale value, the reader should first synchronize with the writers (i.e. via [crate::view::ContextView::wait_until])
    pub fn read(&self, addr: usize, time: Time) -> T {
        let history = self.underlying.get(self.safe_addr(addr)).unwrap();
        let reader = history.read().unwrap();
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
