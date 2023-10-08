use std::cell::RefCell;

use crate::logging::LogInterface;

use super::{LogError, LogEvent};

thread_local! {
    pub static LOGGER: RefCell<Option<LogInterface>> = RefCell::default();
}

#[inline]
pub fn log_event_cb<T: LogEvent, F>(callback: F) -> Result<(), LogError>
where
    F: FnOnce() -> T,
{
    LOGGER.with(|logger| match &*logger.borrow() {
        Some(interface) if interface.log_filter.enabled::<T>() => interface.log(&callback()),
        Some(_) => Ok(()),
        None => Ok(()),
    })
}

#[inline]
pub fn log_event<T: LogEvent>(event: &T) -> Result<(), LogError> {
    LOGGER.with(|logger| match &*logger.borrow() {
        Some(interface) if interface.log_filter.enabled::<T>() => interface.log(event),
        Some(_) => Ok(()),
        None => Ok(()),
    })
}

pub fn initialize_log(logger: LogInterface) {
    LOGGER.with(|cur_logger| {
        let old = cur_logger.replace(Some(logger));
        assert!(matches!(old, None));
    })
}

pub fn copy_log() -> Option<LogInterface> {
    LOGGER.with(|cur_logger| cur_logger.borrow().clone())
}
