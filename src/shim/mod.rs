//! A shim module for users to switch between os-threads and coroutines based on may.

cfg_if::cfg_if! {
    if #[cfg(feature = "os-threads")] {
        mod os_threads;

        pub use os_threads::*;

    }
    else if #[cfg(feature = "coroutines")] {
        mod coroutines;
        pub use coroutines::*;
    }
}


/// Options available when using os threads
/// Execution mode for each thread
#[derive(Debug, Default, Clone, Copy)]
pub enum RunMode {
    /// Execute under the default OS scheduler, such as CFS for Linux
    #[default]
    Simple,

    /// Use FIFO (real-time) scheduling. This is higher performance, but may lead to starvation of other processes.
    FIFO,
}

