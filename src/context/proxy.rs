use dam_core::view::TimeViewable;

use crate::types::Cleanable;

use super::{Context, ContextSummary};

pub enum ProxyContext<T> {
    Running(T),
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
    fn view(&self) -> dam_core::view::TimeView {
        match self {
            ProxyContext::Running(ctx) => ctx.view(),
            ProxyContext::Cleaned(this_summary) => this_summary.time.clone(),
        }
    }
}

impl<T: Context> ProxyContext<T> {
    pub fn summarize(&self) -> ContextSummary {
        match self {
            ProxyContext::Running(_) => panic!("Attempting to summarize a running node!"),
            ProxyContext::Cleaned(summary) => summary.clone(),
        }
    }
}
