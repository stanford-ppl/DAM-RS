use crate::logging::Logger;
use crate::view::TimeManager;

mod identifier;
pub mod sync_unsafe;
mod time;

pub use identifier::*;
pub use time::AtomicTime;
pub use time::Time;

#[derive(Debug, Default, Clone)]
pub enum LogState {
    #[default]
    Disabled,

    Awaiting,
    Active(Logger),
    Terminated,
}

impl LogState {
    pub fn get(&self) -> Option<&Logger> {
        if let LogState::Active(log) = self {
            Some(log)
        } else {
            None
        }
    }
}

#[derive(Default, Debug)]
pub struct ContextInfo {
    pub time: TimeManager,
    pub id: identifier::Identifier,
    pub logger: LogState,
}

#[macro_export]
macro_rules! log_event {
    ($event:expr) => {
        log_event!(self.logger.get(), $event)
    };

    ($logger:expr, $event:expr) => {
        if let Some(logger) = $logger {
            logger.log($event).unwrap();
        }
    };
}
