use cfg_if::cfg_if;

use crate::{datastructures::Time, logging::LogInterface};

use super::{LogError, LogEvent};
cfg_if! {
    if #[cfg(feature = "logging")] {
        use std::cell::RefCell;
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

        pub(crate) fn update_ticks(time: Time) {
            LOGGER.with_borrow_mut(|cur_logger| {
                if let Some(lg) = cur_logger {
                    lg.update_ticks(time);
                }
            })
        }
    } else {
        // Marked as allow(unused) so that we can keep the same signature and names.

        /// No-op without logging enabled
        #[allow(unused)]
        #[inline]
        pub fn log_event_cb<T: LogEvent, F>(callback: F) -> Result<(), LogError> { Ok(()) }

        /// No-op without logging enabled
        #[allow(unused)]
        #[inline]
        pub fn log_event<T: LogEvent>(event: &T) -> Result<(), LogError> { Ok(()) }

        /// No-op without logging enabled
        #[allow(unused)]
        #[inline]
        pub fn initialize_log(logger: LogInterface) {}

        /// No-op without logging enabled
        #[allow(unused)]
        #[inline]
        pub fn copy_log() -> Option<LogInterface> { None }

        /// No-op without logging enabled
        #[allow(unused)]
        #[inline]
        pub(crate) fn update_ticks(time: Time) {}
    }
}
