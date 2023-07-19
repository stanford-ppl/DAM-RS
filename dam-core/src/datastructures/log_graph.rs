use std::{
    cell::RefCell,
    collections::{hash_map::DefaultHasher, HashMap},
    fs::{create_dir_all, File},
    hash::{Hash, Hasher},
    io::{BufWriter, Write},
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
    thread::ThreadId,
    time::{Duration, Instant},
};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::{log_config::get_log_info, metric::NODE, time::Time};

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

impl EventLogger {
    pub fn log<T: std::fmt::Debug>(&self, event: T)
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
    ScopeStart(Identifier, String),
    ScopeEnd(Identifier, String),

    // Registers a child
    WithChild(Identifier, String, Identifier, String),

    Cleaned(Time),
}

pub struct GlobalExecutionRegistry {
    registry: DashMap<ThreadId, Identifier>,
}

impl GlobalExecutionRegistry {
    pub fn get_identifier(&self, thread: ThreadId) -> Option<Identifier> {
        self.registry.get(&thread).map(|x| x.value().clone())
    }

    pub fn register(&self, id: Identifier) -> Option<Identifier> {
        self.registry.insert(std::thread::current().id(), id)
    }

    pub fn unregister(&self) {
        self.registry.remove(&std::thread::current().id());
    }
}

static GLOBAL_REGISTRY: OnceLock<GlobalExecutionRegistry> = OnceLock::new();
pub fn get_registry() -> &'static GlobalExecutionRegistry {
    GLOBAL_REGISTRY.get_or_init(|| GlobalExecutionRegistry {
        registry: DashMap::new(),
    })
}

fn thread_id_to_u64(tid: ThreadId) -> u64 {
    let mut hasher = DefaultHasher::new();
    tid.hash(&mut hasher);
    hasher.finish()
}

// Each thread tracks its own logs.
#[derive(Debug)]
pub struct ThreadLocalLog {
    logs: HashMap<(&'static str, Option<Identifier>), EventLogger>,
    current_context: Option<Identifier>,
}

impl ThreadLocalLog {
    pub fn new() -> Self {
        Self {
            logs: Default::default(),
            current_context: None,
        }
    }

    pub fn get_log(&mut self, key: &'static str) -> EventLogger {
        let lookup_key = &(key, self.current_context);
        match self.logs.get(lookup_key) {
            Some(log) => log.clone(),
            None => {
                let new_log = Self::make_log(key, self.current_context);
                self.logs.insert(*lookup_key, new_log.clone());
                new_log
            }
        }
    }

    fn make_log(key: &'static str, context: Option<Identifier>) -> EventLogger {
        let policy = get_log_info();
        let log = if policy.include.contains(key) {
            let thread_hash = thread_id_to_u64(std::thread::current().id());
            let underlying: Option<Arc<Mutex<BufWriter<File>>>> = policy.path.map(|path| {
                create_dir_all(&path).unwrap();
                let suffix = match context {
                    Some(ctx) => format!("{}_{key}_{thread_hash}.json", ctx),
                    None => format!("NoCtx_{key}_{thread_hash}.json"),
                };
                let full_path = path.join(PathBuf::from(suffix));
                Arc::new(Mutex::new(BufWriter::new(File::create(full_path).unwrap())))
            });
            match underlying {
                Some(arc) => EventLogger {
                    underlying: LogTarget::File(arc, policy.eager_flush),
                },
                None => EventLogger {
                    underlying: LogTarget::Stdout,
                },
            }
        } else {
            EventLogger {
                underlying: LogTarget::Nowhere,
            }
        };

        log
    }
}

thread_local! {
    static LOGS: RefCell<ThreadLocalLog> = RefCell::new(ThreadLocalLog::new());
}

pub fn get_log(key: &'static str) -> EventLogger {
    LOGS.with(|local_logs| local_logs.borrow_mut().get_log(key))
}

pub fn with_log_scope<F: FnOnce()>(id: Identifier, name: String, closure: F) {
    let ident_opt = Some(id);
    LOGS.with(|local_logs| {
        let mut ctx = ident_opt;
        std::mem::swap(&mut local_logs.borrow_mut().current_context, &mut ctx);
        let old_id = get_registry().register(id);
        get_log(NODE).log(RegistryEvent::ScopeStart(id, name.clone()));
        closure();
        get_log(NODE).log(RegistryEvent::ScopeEnd(id, name));
        if let Some(old) = old_id {
            get_registry().register(old);
        }
        std::mem::swap(&mut local_logs.borrow_mut().current_context, &mut ctx);
    })
}
