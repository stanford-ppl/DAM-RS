use crate::{
    types::Cleanable,
    view::{TimeView, TimeViewable},
};

use super::{Context, ContextSummary};

/// A wrapper around a context, to support early dropping.
/// This is used by the PMU and other composite contexts because many events are hooked to the drop.
pub enum ProxyContext<T> {
    /// An actively running (or runnable) context
    Running(T),

    /// A post-execution summary of the context.
    Cleaned(ContextSummary),
}

impl<T: Context> std::ops::Deref for ProxyContext<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        if let ProxyContext::Running(res) = self {
            res
        } else {
            panic!("Attempting to deref a context that's been cleaned already!")
        }
    }
}

impl<T: Context> std::ops::DerefMut for ProxyContext<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if let ProxyContext::Running(res) = self {
            res
        } else {
            panic!("Attempting to deref a context that's been cleaned already!")
        }
    }
}

impl<T: Context> From<T> for ProxyContext<T> {
    fn from(value: T) -> Self {
        ProxyContext::Running(value)
    }
}

impl<T: Context> Cleanable for ProxyContext<T> {
    fn cleanup(&mut self) {
        if let ProxyContext::Running(ctx) = self {
            *self = ProxyContext::Cleaned(ctx.summarize());
        }
    }
}

impl<T: Context> TimeViewable for ProxyContext<T> {
    fn view(&self) -> TimeView {
        match self {
            ProxyContext::Running(ctx) => ctx.view(),
            ProxyContext::Cleaned(this_summary) => this_summary.time.clone(),
        }
    }
}

impl<T: Context> ProxyContext<T> {
    /// Wraps around [Context::summarize]
    pub fn summarize(&self) -> ContextSummary {
        match self {
            ProxyContext::Running(_) => panic!("Attempting to summarize a running node!"),
            ProxyContext::Cleaned(summary) => summary.clone(),
        }
    }
}
