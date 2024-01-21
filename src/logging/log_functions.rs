use std::ops::{Deref, DerefMut};

use cfg_if::cfg_if;

use super::{LogError, LogEvent};
use crate::{datastructures::Time, logging::LogInterface};

// If we're using threads, then each logger needs to be stashed into a thread-local.
// If we're using May/Coroutines instead, then it has to be stashed in coroutine-local instead
cfg_if! {
    if #[cfg(feature = "logging")] {
        cfg_if! {
            if #[cfg(feature = "os-threads")] {
                use std::sync::Mutex;

                thread_local! {
                    static LOGGER: Mutex<Option<LogInterface>> = Default::default();
                }
            } else if #[cfg(feature = "coroutines")] {
                use may::coroutine_local;
                use may::sync::Mutex;
                // We use a mutex here to allow a "static" mutally exclusive object.
                coroutine_local! {
                    static LOGGER: Mutex<Option<LogInterface>> = Default::default()
                }
            }
        }
    }
}

cfg_if! {
    if #[cfg(feature = "logging")] {

        /// Logs with a callback. This should be used when constructing the event is particularly expensive, as it does require extra overhead.
        /// The callback is only invoked if the logger is set AND the filter permits the event.
        #[inline]
        pub fn log_event_cb<T: LogEvent, F>(callback: F) -> Result<(), LogError>
        where
            F: FnOnce() -> T,
        {
            LOGGER.with(|logger| match logger.lock().unwrap().deref() {
                Some(interface) if interface.log_filter.enabled::<T>() => interface.log(&callback()),
                Some(_) => Ok(()),
                None => Ok(()),
            })
        }

        /// Standard logging method, which logs to the underlying logger.
        #[inline]
        pub fn log_event<T: LogEvent>(event: &T) -> Result<(), LogError> {
            LOGGER.with(|logger| match logger.lock().unwrap().deref() {
                Some(interface) if interface.log_filter.enabled::<T>() => interface.log(event),
                Some(_) => Ok(()),
                None => Ok(()),
            })
        }

        /// Initializes the thread-local log with a specific logger.
        pub fn initialize_log(logger: LogInterface) {
            LOGGER.with(|lg| {*lg.lock().unwrap() = Some(logger);})
        }

        /// Gets the current logger, used for 'inheriting' loggers from a parent context.
        pub fn copy_log() -> Option<LogInterface> {
            LOGGER.with(|cur_logger| cur_logger.lock().unwrap().clone())
        }

        pub(crate) fn update_ticks(time: Time) {
            LOGGER.with(|cur_logger| {
                if let Some(lg) = cur_logger.lock().unwrap().deref_mut() {
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
