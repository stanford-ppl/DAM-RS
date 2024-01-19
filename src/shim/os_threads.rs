/// Re-exports for channel behaviors
pub mod channel {
    pub use crossbeam::channel::*;
}

pub use std::thread::current;
pub use std::thread::scope;
pub use thread_priority::ThreadBuilder as Builder;

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
pub fn make_builder(mode: RunMode) -> Builder {
    let (priority, policy) = match mode {
        RunMode::Simple => (
            thread_priority::get_current_thread_priority().unwrap(),
            thread_priority::thread_schedule_policy().unwrap(),
        ),
        RunMode::FIFO => {
            let priority = thread_priority::ThreadPriority::Crossplatform(10u8.try_into().unwrap());
            let policy = thread_priority::unix::ThreadSchedulePolicy::Realtime(
                thread_priority::RealtimeThreadSchedulePolicy::Fifo,
            );
            (priority, policy)
        }
    };
    thread_priority::ThreadBuilder::default()
        .priority(priority)
        .policy(policy)
}

/// Shim around builder
pub fn spawn<'scope, 'env, F, T>(
    scope: &'scope std::thread::Scope<'scope, 'env>,
    builder: Builder,
    f: F,
) -> Result<std::thread::ScopedJoinHandle<'scope, T>, std::io::Error>
where
    F: FnOnce() -> T,
    F: Send + 'scope,
    T: Send + 'scope,
{
    builder.spawn_scoped_careless(scope, f)
}
