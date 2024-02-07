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

pub use may::coroutine::current;
pub use may::coroutine::scope;
pub use may::coroutine::Builder;

pub use may::coroutine::park;
pub use may::coroutine::sleep;
pub use may::coroutine::yield_now;
pub use may::coroutine::Coroutine as Thread;
pub use may::coroutine_local as local_storage;

/// Constructs a thread builder based on the options specified in the [RunMode]
pub fn make_builder(_mode: super::RunMode) -> Builder {
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
