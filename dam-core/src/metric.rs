use std::{cell::RefCell, collections::HashMap};

use linkme::distributed_slice;

use crate::log_graph::{self, WeakEventLogger};

thread_local! {
    static LOG_CACHE: RefCell<HashMap<&'static str, WeakEventLogger>> = RefCell::new(HashMap::new());
}

pub trait LogProducer {
    // This should be a simple name consisting only of [a-zA-Z] and "-" characters.
    const LOG_NAME: &'static str;

    fn log<T: serde::Serialize + std::fmt::Debug>(value: T) {
        LOG_CACHE.with(|cache: &RefCell<HashMap<&str, WeakEventLogger>>| {
            if !cache.borrow().contains_key(Self::LOG_NAME) {
                let thread_id = std::thread::current().id();
                let current_graph = crate::log_graph::get_graph();
                let identifier = current_graph.get_identifier(thread_id);
                let log = current_graph.get_log(log_graph::LogType::Event(
                    identifier,
                    thread_id,
                    Self::LOG_NAME,
                ));
                cache.borrow_mut().insert(Self::LOG_NAME, log.weak());
            }

            cache
                .borrow()
                .get(Self::LOG_NAME)
                .expect("We should have gotten a log!")
                .promote()
                .expect("We're not dead yet, why is our log?")
                .log(value);
        });
    }
}

pub fn validate_name<'a>(name: &'a str) -> bool {
    METRICS.contains(&name)
}

#[distributed_slice]
pub static METRICS: [&'static str] = [..];

// Gathers all information about the nodes
#[distributed_slice(METRICS)]
pub static NODE: &'static str = "NODE";
