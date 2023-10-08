pub mod channel;
pub mod context;
pub mod simulation;
pub mod templates;
pub mod types;
pub use dam_macros;
pub mod utility_contexts;

mod datastructures;
mod view;

pub mod logging;

// utility grouping for all the peripherals
pub mod structures {
    pub use crate::datastructures::*;
    pub use crate::view::*;
}

// Re-exports the common structures needed to build contexts.
pub mod context_tools {
    // Used for reading/writing to channels, and to take/use them as members
    pub use crate::channel::{ChannelElement, Receiver, Sender};

    pub use crate::types::DAMType;
    pub use dam_macros::context_macro;

    pub use crate::logging::{log_event, log_event_cb};

    pub use crate::context::Context;

    pub use crate::view::ContextView;
}

// Re-exports structures for macro use
pub mod macro_support {
    pub use crate::datastructures::{ContextInfo, Identifiable, Identifier};
    pub use crate::logging;
    pub use crate::view::{TimeManager, TimeView, TimeViewable};
}
