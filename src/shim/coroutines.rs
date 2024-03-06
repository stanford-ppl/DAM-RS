/// Re-exports for channel behaviors
pub mod channel {
    pub use may::sync::spsc::channel as unbounded;
    pub use may::sync::spsc::*;

    /// Simple shim for bounded around unbounded.
    pub fn bounded<T>(_: usize) -> (Sender<T>, Receiver<T>) {
        may::sync::spsc::channel()
    }

    pub use std::sync::mpsc::TryRecvError;
}

pub use may::config;
pub use may::coroutine::current;
pub use may::coroutine::scope;
pub use may::coroutine::Builder;

pub use may::coroutine::park;
pub use may::coroutine::sleep;
pub use may::coroutine::yield_now;
pub use may::coroutine::Coroutine as Thread;
pub use may::coroutine_local as local_storage;
pub use may::sync::{Condvar, Mutex, RwLock};

/// Options available when using os threads
/// Execution mode for each thread
#[derive(Debug, Default, Clone, Copy)]
pub enum RunMode {
    /// Execute under the default OS scheduler, such as CFS for Linux
    #[default]
    Simple,

    /// Deprecated, supported for compatibility purposes.
    #[deprecated(
        note = "FIFO mode is a language-level compatibility shim for existing applications. New applications should use Simple."
    )]
    FIFO,

    /// Use a fixed maximum number of workers
    Constrained(usize),
}

/// Constructs a thread builder based on the options specified in the [RunMode]
pub fn make_builder(mode: super::RunMode) -> Builder {
    if let RunMode::Constrained(workers) = mode {
        config().set_workers(workers);
    }
    let num_workers = config().get_workers();
    let target = fastrand::usize(0..num_workers);
    may::coroutine::Builder::new().id(target)
}

/// Spawns a coroutine, without the builder because
#[macro_export]
macro_rules! spawn {
    ($scope: expr, $builder: expr, $f: expr) => {{
        unsafe { ($scope).spawn_with_builder($f, $builder) };
        Result::<(), ()>::Ok(())
    }};
    ($scope: expr, $f: expr) => {{
        unsafe { ($scope).spawn($f) };
        Result::<(), ()>::Ok(())
    }};
}

pub use spawn;
