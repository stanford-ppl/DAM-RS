/// Re-exports for channel behaviors
pub mod channel {
    pub use may::sync::mpmc::channel as unbounded;
    pub use may::sync::mpmc::*;

    /// Simple shim for bounded around unbounded.
    pub fn bounded<T>(_: usize) -> (Sender<T>, Receiver<T>) {
        may::sync::mpmc::channel()
    }

    pub use std::sync::mpsc::TryRecvError;
}

pub use may::coroutine::current;
pub use may::coroutine::scope;
pub use may::coroutine::Builder;

pub use may::coroutine::park;
pub use may::coroutine::sleep;
pub use may::coroutine::yield_now;
pub use may::coroutine::Coroutine as Thread;

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

/// Constructs a thread builder based on the options specified in the [RunMode]
pub fn make_builder(_mode: RunMode) -> Builder {
    may::coroutine::Builder::new()
}

/// Spawns a coroutine, without the builder because
#[macro_export]
macro_rules! spawn {
    ($scope: expr, $builder: expr, $f: expr) => {{
        let _ = $builder;
        unsafe { ($scope).spawn($f) };
        Result::<(), ()>::Ok(())
    }};
    ($scope: expr, $f: expr) => {{
        unsafe { ($scope).spawn($f) };
        Result::<(), ()>::Ok(())
    }};
}

pub use spawn;
