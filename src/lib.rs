#![warn(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! The Dataflow Abstract Machine is a simulation framework designed for simulating dataflow-like systems.
//! In particular, DAM optimizes for complex nodes connected by statically known channels.
//! For more details, see (paper not yet published).

pub mod channel;
pub mod context;
mod datastructures;
pub mod logging;
pub mod simulation;
pub mod types;
pub mod utility_contexts;
mod view;

pub use dam_macros;

/// Reference implementations of a few basic components.
pub mod templates;

/// Utility grouping for common constructs
pub mod structures {
    pub use crate::datastructures::*;
    pub use crate::view::*;
}

/// Re-exports the common structures needed to build contexts.
/// For simple contexts, this should be all that is needed.
pub mod context_tools {
    pub use crate::channel::{ChannelElement, Receiver, Sender};

    pub use crate::types::DAMType;
    pub use dam_macros::context_macro;

    pub use crate::logging::{log_event, log_event_cb};

    pub use crate::context::Context;

    pub use crate::view::ContextView;
}

/// Re-exports structures for macro use.
/// These macros are used by dam-macros, without needing to complicate macros with the complete paths.
/// These should not be depended on by anyone to be consistent across versions.
#[doc(hidden)]
pub mod macro_support {
    pub use crate::datastructures::{ContextInfo, Identifiable, Identifier};
    pub use crate::logging;
    pub use crate::view::{TimeManager, TimeView, TimeViewable};
}
