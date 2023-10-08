use std::cell::RefCell;

use crate::logging::LogInterface;

use super::{LogError, LogEvent};

thread_local! {
    pub(crate) static LOGGER: RefCell<Option<LogInterface>> = RefCell::default();
}

/// Logs with a callback. This should be used when constructing the event is particularly expensive, as it does require extra overhead.
/// The callback is only invoked if the logger is set AND the filter permits the event.
#[inline]
pub fn log_event_cb<T: LogEvent, F>(callback: F) -> Result<(), LogError>
where
    F: FnOnce() -> T,
{
    LOGGER.with_borrow(|logger| match logger {
        Some(interface) if interface.log_filter.enabled::<T>() => interface.log(&callback()),
        Some(_) => Ok(()),
        None => Ok(()),
    })
}

/// Standard logging method, which logs to the underlying logger.
#[inline]
pub fn log_event<T: LogEvent>(event: &T) -> Result<(), LogError> {
    LOGGER.with_borrow(|logger| match logger {
        Some(interface) if interface.log_filter.enabled::<T>() => interface.log(event),
        Some(_) => Ok(()),
        None => Ok(()),
    })
}

/// Initializes the thread-local log with a specific logger.
pub fn initialize_log(logger: LogInterface) {
    LOGGER.set(Some(logger));
}

/// Gets the current logger, used for 'inheriting' loggers from a parent context.
pub fn copy_log() -> Option<LogInterface> {
    LOGGER.with_borrow(|cur_logger| cur_logger.clone())
}
