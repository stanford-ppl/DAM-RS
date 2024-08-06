//! Context provides the basic traits and wrappers for defining the behavior of logical units.
//! Each context is programmed in a CSP-like fashion, expressing its entire execution via a monolithic [Context::run] method, which accepts arbitrary user code.

use std::{
    collections::{HashMap, HashSet},
    panic::AssertUnwindSafe,
};

use crate::{
    channel::ChannelID,
    datastructures::{Identifiable, Identifier, VerboseIdentifier},
    view::TimeViewable,
};

mod proxy;

mod summary;
pub use summary::ContextSummary;

pub use proxy::ProxyContext;
use thiserror::Error;

/// Explicitly specified connections from input channels to output channels -- used for arbiter-like contexts.
pub type ExplicitConnections = HashMap<Identifier, Vec<(HashSet<ChannelID>, HashSet<ChannelID>)>>;

/// RuntimeError encompasses errors which may occur when using the [Context::run_falliable] shim around [Context::run].
/// In particular, the conversion between mutable references and pointers may fail, or the run function itself may panic.
#[derive(Error, Debug)]
pub enum RuntimeError {
    /// ContextErrors reflect panics in the original run function.
    #[error("Panic occurred")]
    ContextError,

    /// WrapErrors reflect a failure in the ref -> ptr -> ref process.
    #[error("Run shim failed")]
    WrapError,
}

/// The core component of DAM.
pub trait Context: Send + Sync + TimeViewable + Identifiable {
    // A lot of contexts simply define this to be empty anyways.
    /// Initializes the context -- frequently a no-op. No guarantee that initialization is executed in the same thread as the run operation currently.
    fn init(&mut self) {}

    /// The 'meat-and-bones' of a context, expressed as a monolithic function.
    #[deprecated]
    fn run(&mut self) {
        panic!("Run wasn't implemented!")
    }

    /// A falliable version of [Context::run]
    fn run_falliable(&mut self) -> anyhow::Result<()> {
        // Shim around the run method. New versions should use run_falliable.
        let self_ptr = std::ptr::from_mut(self);
        let execution = std::panic::catch_unwind(AssertUnwindSafe(|| {
            // Stuff
            match unsafe { self_ptr.as_mut() } {
                Some(newself) => {
                    #[allow(deprecated)]
                    newself.run();
                    Ok(())
                }
                None => Err(()),
            }
        }));
        match execution {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(_)) => {
                // This is an error from the unsafe block above.
                Err(RuntimeError::WrapError)?
            }
            Err(_) => {
                // This is a panic from the catch_unwind
                Err(RuntimeError::ContextError)?
            }
        }
    }

    /// A map of IDs to child IDs. For most nodes this will be ID -> Empty.
    fn ids(&self) -> HashMap<VerboseIdentifier, HashSet<VerboseIdentifier>> {
        HashMap::from([(self.verbose(), HashSet::new())])
    }

    /// By default all edges are connected.
    /// In the case of something like a PMU, however, we wish to be finer-grained than that.
    /// In that case, we can report channel A -> {B, C, D} means that A sends data that can be observed on B, C, and/or D.
    fn edge_connections(&self) -> Option<ExplicitConnections> {
        None
    }

    /// Returns a summary of the context, which is then dropped by the programgraph.
    fn summarize(&self) -> ContextSummary {
        ContextSummary {
            id: self.verbose(),
            time: self.view(),
            children: vec![],
        }
    }
}
