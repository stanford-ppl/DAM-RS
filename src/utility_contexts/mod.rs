//! Common utility contexts for testing, checking, etc.
//! These can be used to 'black-box' a lot of functionality, such as reading from a file for input, etc.

mod approx_checker_context;
mod broadcast_context;
mod checker_context;
mod consumer_context;
mod function_context;
mod generator_context;
mod trace_context;

use std::fmt::Debug;

pub use approx_checker_context::ApproxCheckerContext;
pub use broadcast_context::BroadcastContext;
pub use checker_context::CheckerContext;
pub use consumer_context::{ConsumerContext, PrinterContext};
pub use function_context::FunctionContext;
pub use generator_context::GeneratorContext;
use thiserror::Error;
pub use trace_context::{random_trace, TraceContext};

use crate::channel::ChannelID;

/// A bundle of generic failures for utility contexts.
#[derive(Debug, Error)]
pub enum UtilityError {
    /// A receiver was prematurely closed, which caused the current context to fail.
    #[error("Receiver was prematurely closed")]
    Receiver {
        /// The logical loop iteration when the failure occurred
        iteration: usize,
        /// The channel ID of the failure
        channel: ChannelID,
    },

    /// A context was somehow executed twice
    #[error("Cannot execute a context more than once!")]
    DuplicateExec,
}

/// Checker failures
#[derive(Debug, Error)]
pub enum CheckerError {
    /// Mismatch between expected and actual values
    #[error("Mismatched results on iteration {ind:?}: {msg}")]
    Mismatch {
        /// The index of the mismatch
        ind: usize,

        /// The error message. Conversion must happen early in case T contains a reference.
        msg: String,
    },
}
