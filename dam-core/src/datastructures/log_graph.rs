use std::{
    collections::{hash_map::DefaultHasher, HashSet, VecDeque},
    fs::File,
    hash::{Hash, Hasher},
    io::{BufWriter, Write},
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
    thread::ThreadId,
};

use dashmap::{DashMap, DashSet};
use serde::{Deserialize, Serialize};

use crate::identifier::{self, Identifiable};

use super::identifier::Identifier;

#[derive(Clone, Default, Debug)]
pub struct EventLogger {
    underlying: Option<Arc<Mutex<BufWriter<File>>>>,
}

impl EventLogger {
    pub fn log<T: std::fmt::Debug>(&mut self, event: T)
    where
        T: serde::Serialize,
    {
        match &self.underlying {
            Some(wr) => {
                let mut writer = wr.lock().unwrap();
                writer
                    .write_all(
                        serde_json::to_string(&event)
                            .expect("Failed to serialize struct!")
                            .as_bytes(),
                    )
                    .unwrap();
                writer.write_all("\n".as_bytes()).unwrap();
            }
            None => println!("{:?}", event),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RegistryEvent {
    // Registers the name of this node
    Created(String),

    // Registers the parent identifier of this node.
    WithParent(Identifier),
}

#[derive(Default, Debug)]
pub struct LogGraph {
    // Maps children to their parents
    child_parent_tree: DashMap<Identifier, Identifier>,

    // Maps threads to identifiers
    executor_map: DashMap<ThreadId, Identifier>,

    // Maps threads to file paths
    logging_paths: DashMap<Identifier, PathBuf>,
    // All identifiers
    all_identifiers: DashSet<Identifier>,

    // Loggers are per-thread to save on overhead
    loggers: DashMap<LogType, EventLogger>,
}

impl LogGraph {
    pub fn dump(&self) {
        for pair in self.child_parent_tree.iter() {
            println!("Child({:?}) -> Parent({:?})", pair.key(), pair.value());
        }

        for id in self.all_identifiers.iter() {
            if !self.child_parent_tree.contains_key(id.key()) {
                println!("Orphan ID: {:?}", id.key());
            }
        }
    }

    pub fn add_id(&self, id: Identifier) {
        self.all_identifiers.insert(id);
    }
}

pub struct LogGraphHandle<'a> {
    self_id: Identifier,
    under: &'a LogGraph,
}

impl<'a> LogGraphHandle<'a> {
    pub fn add_child(&mut self, child: Identifier) {
        self.under.child_parent_tree.insert(child, self.self_id);
    }
}

pub fn get_graph() -> &'static LogGraph {
    GRAPH.get_or_init(Default::default)
}

static GRAPH: OnceLock<LogGraph> = OnceLock::new();

pub fn register_handle<'a>(self_id: Identifier) -> LogGraphHandle<'a> {
    let g = get_graph();
    LogGraphHandle { self_id, under: g }
}

fn thread_id_to_u64(id: ThreadId) -> u64 {
    // TODO: Replace this with just getting the u64 of the thread id once as_u64 is stabilized.
    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish()
}

// Returns the directory that this compute tree is supposed to be using.
pub fn get_log_path(self_id: Identifier) -> Option<PathBuf> {
    let g = get_graph();
    let mut cur_thread = self_id;
    loop {
        match g.logging_paths.get(&cur_thread) {
            // One of our parents had a registered path!
            // This shouldn't happen very often.
            Some(path) => return Some(path.clone()),
            None => {
                let tree_handle = &g.child_parent_tree;
                match tree_handle.get(&cur_thread) {
                    // Nothing matched, but we do have a parent, so climb the tree!
                    Some(parent) => cur_thread = parent.clone(),

                    // We're the parent, and we don't have anything to work with.
                    None => return None,
                }
            }
        }
    }
}

pub fn set_log_path(id: Identifier, path: PathBuf) {
    let g = GRAPH.get_or_init(Default::default);
    let paths = &g.logging_paths;

    // It's fine to fail on the remove_dir_all, since the path might not exist.
    let _ = std::fs::remove_dir_all(path.clone());

    // This one isn't allowed to fail since we're going to be writing there.
    std::fs::create_dir_all(path.clone()).unwrap();

    paths.insert(id, path);
}

// Path format:
// Base log: name_identifier only
// Event logs: name_identifier_thread_logsrc

#[derive(Clone, PartialEq, Hash, Eq, Debug)]
pub enum LogType {
    // Context name, identifier
    Base(Identifier),

    // Context name, identifier, Thread id, type
    Event(Identifier, ThreadId, String),
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

pub fn get_log(key: LogType) -> EventLogger {
    let entry = get_graph().loggers.entry(key.clone());
    let logger = entry.or_insert_with(|| {
        let full_path = get_log_path(key.id()).map(|p| p.join(key.to_path()));
        let underlying =
            full_path.map(|path| Arc::new(Mutex::new(BufWriter::new(File::create(path).unwrap()))));
        EventLogger { underlying }
    });
    logger.value().clone()
}

pub fn get_identifier(thread: ThreadId) -> Identifier {
    *get_graph().executor_map.get(&thread).unwrap().value()
}

pub fn register(id: Identifier, name: String) {
    let graph = get_graph();
    graph.executor_map.insert(std::thread::current().id(), id);

    get_log(LogType::Base(id)).log(RegistryEvent::Created(name));
}

pub fn is_orphan(identifier: Identifier) -> bool {
    !get_graph().child_parent_tree.contains_key(&identifier)
}

fn get_subgraph(root: Identifier, graph: &LogGraph) -> HashSet<Identifier> {
    let mut subgraph = HashSet::new();
    subgraph.insert(root);

    let mut to_process = VecDeque::<Identifier>::new();

    for identifier in graph.all_identifiers.iter() {
        to_process.push_back(*identifier.key());
    }

    loop {
        match to_process.pop_front() {
            Some(identifier) => {
                // if the identifier's parent is in the subgraph, then we add identifier as well
                match graph.child_parent_tree.get(&identifier) {
                    Some(parent) if subgraph.contains(parent.value()) => {
                        subgraph.insert(identifier);
                    }

                    // We don't know about the parent yet, we'll delay until later.
                    Some(_) => to_process.push_back(identifier),
                    None => {} // This one's an orphan, it's done,
                }
            }
            None => break,
        }
    }

    subgraph
}

pub fn drop_subgraph(root: Identifier) {
    let g = get_graph();
    let can_drop = get_subgraph(root, g);
    g.all_identifiers
        .retain(|identifier| !can_drop.contains(identifier));

    g.child_parent_tree.retain(|id, _| !can_drop.contains(id));
    g.executor_map.retain(|_, v| !can_drop.contains(v));
    g.logging_paths.retain(|k, _| !can_drop.contains(k));
    g.loggers.retain(|k, _| !can_drop.contains(&k.id()));
}

#[macro_export]
macro_rules! DAMLog {
    () => {
        ({
            let thread_id = ::std::thread::current().id();
            let identifier = ::dam_core::log_graph::get_identifier(thread_id);
            let typename = ::std::any::type_name::<Self>();
            ::dam_core::log_graph::get_log(::dam_core::log_graph::LogType::Event(
                identifier,
                thread_id,
                typename.into(),
            ))
        })
    };
}

#[macro_export]
macro_rules! DAMLog_core {
    () => {
        ({
            let thread_id = ::std::thread::current().id();
            let identifier = crate::log_graph::get_identifier(thread_id);
            let typename = ::std::any::type_name::<Self>();
            crate::log_graph::get_log(crate::log_graph::LogType::Event(
                identifier,
                thread_id,
                typename.into(),
            ))
        })
    };
}
