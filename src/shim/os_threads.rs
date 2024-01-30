/// Re-exports for channel behaviors
pub mod channel {
    pub use crossbeam::channel::*;
}

pub use std::thread::current;
pub use std::thread::park;
pub use std::thread::scope;
pub use std::thread::sleep;
pub use std::thread::yield_now;
pub use std::thread::Thread;
pub use thread_priority::ThreadBuilder as Builder;

/// Constructs a thread builder based on the options specified in the [RunMode]
pub fn make_builder(mode: super::RunMode) -> Builder {
    let (priority, policy) = match mode {
        super::RunMode::Simple => (
            thread_priority::get_current_thread_priority().unwrap(),
            thread_priority::thread_schedule_policy().unwrap(),
        ),
        super::RunMode::FIFO => {
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

/// Spawns a coroutine, without the builder because
#[macro_export]
macro_rules! spawn {
    ($scope: expr, $builder: expr, $f: expr) => {
        ($builder).spawn_scoped_careless($scope, $f)
    };

    ($scope: expr, $f: expr) => {{
        ($scope).spawn($f);
        Result::<(), ()>::Ok(())
    }};
}

pub use spawn;
