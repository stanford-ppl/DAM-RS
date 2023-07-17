use std::{
    collections::{hash_map::DefaultHasher, HashSet, VecDeque},
    fs::{create_dir_all, File},
    hash::{Hash, Hasher},
    io::{BufWriter, Write},
    path::PathBuf,
    sync::{Arc, Mutex, Once, OnceLock},
    thread::ThreadId,
    time::{Duration, Instant},
};

use dashmap::{DashMap, DashSet};
use serde::{Deserialize, Serialize};

use crate::{
    config::get_config,
    log_config::{get_log_info, LogInfo},
    metric::NODE,
    time::Time,
};

use super::identifier::Identifier;

#[derive(Debug, Clone)]
enum LogTarget {
    // File handle, eager flush.
    File(Arc<Mutex<BufWriter<File>>>, bool),
    Stdout,
    Nowhere,
}
#[derive(Clone, Debug)]
pub struct EventLogger {
    underlying: LogTarget,
}

static INIT_TIME: OnceLock<Instant> = OnceLock::new();

fn time_since_init() -> Duration {
    Instant::now() - *INIT_TIME.get_or_init(Instant::now)
}

static LOG_INFO: OnceLock<LogInfo> = OnceLock::new();
fn get_base_policy() -> LogInfo {
    LOG_INFO
        .get_or_init(|| get_config().log_config.clone().try_into().unwrap())
        .clone()
}

impl EventLogger {
    pub fn log<T: std::fmt::Debug>(&mut self, event: T)
    where
        T: serde::Serialize,
    {
        match &self.underlying {
            LogTarget::File(wr, flush) => {
                let mut writer = wr.lock().unwrap();
                let time_str = format!("[{}]\t", time_since_init().as_micros());
                writer.write_all(time_str.as_bytes()).unwrap();
                writer
                    .write_all(
                        serde_json::to_string(&event)
                            .expect("Failed to serialize struct!")
                            .as_bytes(),
                    )
                    .unwrap();
                writer.write_all("\n".as_bytes()).unwrap();
                if *flush {
                    writer.flush().unwrap();
                }
            }
            LogTarget::Stdout => println!("{:?}", event),
            LogTarget::Nowhere => {}
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RegistryEvent {
    // Registers the name of this node
    Created(String),

    // Registers the parent identifier of this node.
    WithParent(Identifier),

    // Cleaned up, only registered on the topmost
    Cleaned(u128, Time),
}

#[derive(Default, Debug)]
pub struct LogGraph {
    // Maps children to their parents
    child_parent_tree: DashMap<Identifier, Identifier>,

    parent_child_map: DashMap<Identifier, Vec<Identifier>>,

    // Maps threads to identifiers
    executor_map: DashMap<ThreadId, Identifier>,

    // Maps threads to file paths
    logging_paths: DashMap<Identifier, LogInfo>,
    // All identifiers
    all_identifiers: DashSet<Identifier>,

    // Loggers are per-thread to save on overhead
    loggers: DashMap<LogType, EventLogger>,
}

impl LogGraph {
    pub fn add_id(&self, id: Identifier) {
        self.all_identifiers.insert(id);
    }

    pub fn register_handle<'a>(&'a self, self_id: Identifier) -> LogGraphHandle<'a> {
        LogGraphHandle {
            self_id,
            under: self,
        }
    }

    // Returns the directory that this compute tree is supposed to be using.
    pub fn get_log_path(&self, self_id: Identifier) -> Option<PathBuf> {
        let helper = || {
            let mut cur_thread = self_id;
            loop {
                let log_path = self
                    .logging_paths
                    .get(&cur_thread)
                    .map(|k| k.value().clone());
                match log_path {
                    // One of our parents had a registered path!
                    // This shouldn't happen very often.
                    Some(parent_log) => return parent_log.path,
                    None => {
                        let tree_handle = &self.child_parent_tree;
                        match tree_handle.get(&cur_thread) {
                            // Nothing matched, but we do have a parent, so climb the tree!
                            Some(parent) => cur_thread = parent.value().clone(),

                            // We're the parent, and we don't have anything to work with.
                            None => return None,
                        }
                    }
                }
            }
        };
        let part = helper();
        let log_info = get_log_info();
        match (&log_info.path, &part) {
            (None, None) => None,
            (None, Some(_)) => part,
            (Some(_), None) => log_info.path.clone(),
            (Some(head), Some(tail)) => Some(head.join(tail)),
        }
    }

    pub fn set_log_info(&self, id: Identifier, log_info: LogInfo) {
        let paths = &self.logging_paths;

        paths.insert(id, log_info);
    }

    pub fn get_log_policy(&self, key: LogType) -> LogInfo {
        // traverse up the parent tree. Policies encountered here are considered overrides.
        let mut id = key.id();
        loop {
            let info = self.logging_paths.get(&id).map(|x| x.value().clone());
            if let Some(result) = info {
                return result;
            }
            match self.child_parent_tree.get(&id) {
                Some(parent) => id = parent.value().clone(),
                None => return get_base_policy(),
            }
        }
    }

    pub fn get_log(&self, key: LogType) -> EventLogger {
        let entry = self.loggers.entry(key.clone());
        let logger = entry.or_insert_with(|| {
            let policy = self.get_log_policy(key);
            if let LogType::Event(_, _, tp) = key {
                if !policy.include.contains(&tp.to_string()) {
                    // Empty EventLogger
                    return EventLogger {
                        underlying: LogTarget::Nowhere,
                    };
                }
            }
            if let LogType::Base(_) = key {
                if !policy.include.contains(NODE) {
                    return EventLogger {
                        underlying: LogTarget::Nowhere,
                    };
                }
            }
            let full_path = self.get_log_path(key.id()).map(|p| {
                create_dir_all(&p).unwrap();
                p.join(key.to_path())
            });
            let underlying = full_path
                .map(|path| Arc::new(Mutex::new(BufWriter::new(File::create(path).unwrap()))));
            match underlying {
                Some(arc) => EventLogger {
                    underlying: LogTarget::File(arc, policy.eager_flush),
                },
                None => EventLogger {
                    underlying: LogTarget::Stdout,
                },
            }
        });
        logger.value().clone()
    }

    pub fn get_identifier(&self, thread: ThreadId) -> Identifier {
        *self.executor_map.get(&thread).unwrap().value()
    }

    pub fn register(&self, id: Identifier, name: String) {
        self.executor_map.insert(std::thread::current().id(), id);

        self.get_log(LogType::Base(id))
            .log(RegistryEvent::Created(name));
    }

    pub fn is_orphan(&self, identifier: Identifier) -> bool {
        self.child_parent_tree.contains_key(&identifier)
    }

    fn get_subgraph(&self, root: Identifier) -> HashSet<Identifier> {
        let mut subgraph = HashSet::new();

        let mut to_process = VecDeque::<Identifier>::new();
        to_process.push_back(root);
        while !to_process.is_empty() {
            let next = to_process.pop_front().unwrap();
            let already_exists = subgraph.contains(&next);
            if !already_exists {
                subgraph.insert(next);
                let children = self.parent_child_map.get(&next);
                if let Some(r) = children {
                    to_process.extend(r.value().clone());
                }
            }
        }

        subgraph
    }

    pub fn drop_subgraph(&self, root: Identifier, time: Time) {
        self.get_log(LogType::Base(root))
            .log(RegistryEvent::Cleaned(time_since_init().as_micros(), time));
        let can_drop = self.get_subgraph(root);
        self.all_identifiers
            .retain(|identifier| !can_drop.contains(identifier));

        self.child_parent_tree
            .retain(|id, _| !can_drop.contains(id));
        self.executor_map.retain(|_, v| !can_drop.contains(v));
        self.logging_paths.retain(|k, _| !can_drop.contains(k));
        self.loggers.retain(|k, _| !can_drop.contains(&k.id()));
    }
}

pub struct LogGraphHandle<'a> {
    self_id: Identifier,
    under: &'a LogGraph,
}

impl<'a> LogGraphHandle<'a> {
    pub fn add_child(&mut self, child: Identifier) {
        self.under.child_parent_tree.insert(child, self.self_id);
        let pcm_entry = self.under.parent_child_map.entry(self.self_id);
        pcm_entry.or_insert_with(Vec::new).push(child);
    }
}

pub fn get_graph() -> &'static LogGraph {
    // Initialize time here as well
    INIT_TIME.get_or_init(Instant::now);
    let result = GRAPH.get_or_init(Default::default);
    start_static_hook();
    result
}

static PANIC_HOOK: Once = Once::new();
fn start_static_hook() {
    PANIC_HOOK.call_once(|| {
        // Also hook into the global panic chain.
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let graph = get_graph();
            let cur_thread = std::thread::current().id();
            let identifier = graph
                .executor_map
                .get(&cur_thread)
                .map(|ident| ident.value().clone());
            if let Some(ident) = identifier {
                get_graph().drop_subgraph(ident, Time::new(0));
            }
            (prev_hook)(panic_info);
        }));
    });
}

static GRAPH: OnceLock<LogGraph> = OnceLock::new();

fn thread_id_to_u64(id: ThreadId) -> u64 {
    // TODO: Replace this with just getting the u64 of the thread id once as_u64 is stabilized.
    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish()
}

// Path format:
// Base log: name_identifier only
// Event logs: name_identifier_thread_logsrc

#[derive(Clone, PartialEq, Hash, Eq, Debug, Copy)]
pub enum LogType {
    // Context name, identifier
    Base(Identifier),

    // Context name, identifier, Thread id, type
    Event(Identifier, ThreadId, &'static str),
}

impl LogType {
    fn to_path(&self) -> PathBuf {
        match self {
            LogType::Base(i) => format!("{i}.json").into(),
            LogType::Event(i, thread, tp) => {
                format!("{i}_{}_{tp}.json", thread_id_to_u64(*thread)).into()
            }
        }
    }

    fn id(&self) -> Identifier {
        match self {
            LogType::Base(id) => *id,
            LogType::Event(id, _, _) => *id,
        }
    }
}
