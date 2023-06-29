use crate::{time::Time, types::DAMType};

pub struct DRAMConfig {
    num_simultaneous_requests: usize,
    bandwidth_in_bits: usize,
}

// The basic DRAM handles scalar addressing
pub struct DRAM<T: DAMType> {
    config: DRAMConfig,
    datastore: Vec<T>,
    initial_value: Option<T>,
    // A rotating buffer for when each request window opens up
    request_windows: Vec<Time>,
}
