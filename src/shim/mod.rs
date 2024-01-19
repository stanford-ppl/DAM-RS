//! A shim module for users to switch between os-threads and coroutines based on may.

cfg_if::cfg_if! {
    if #[cfg(feature = "os-threads")] {
        mod os_threads;

        pub use os_threads::*;

    }
}
