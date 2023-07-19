use std::{
    cell::RefCell,
    collections::HashMap,
    fs::{create_dir_all, File},
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
    Created(String),

    // Registers a child
    WithChild(Identifier, String),

    Executing(String),

    Cleaned(Time),
}

pub struct GlobalExecutionRegistry {
    registry: DashMap<ThreadId, Identifier>,
}

impl GlobalExecutionRegistry {
    pub fn get_identifier(&self, thread: ThreadId) -> Option<Identifier> {
        self.registry.get(&thread).map(|x| x.value().clone())
    }

    pub fn register(&self, id: Identifier, name: String) {
        let current = std::thread::current().id();
        if self.registry.contains_key(&current) {
            return;
        }
        self.registry.insert(std::thread::current().id(), id);
        set_task(id, name);
    }
}

static GLOBAL_REGISTRY: OnceLock<GlobalExecutionRegistry> = OnceLock::new();
pub fn get_registry() -> &'static GlobalExecutionRegistry {
    GLOBAL_REGISTRY.get_or_init(|| GlobalExecutionRegistry {
        registry: DashMap::new(),
    })
}

// Each thread tracks its own logs.
#[derive(Debug)]
pub struct ThreadLocalLog {
    logs: HashMap<&'static str, EventLogger>,
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
        match self.logs.get(key) {
            Some(log) => log.clone(),
            None => {
                let new_log = self.make_log(key);
                self.logs.insert(key, new_log.clone());
                new_log
            }
        }
    }

    fn make_log(&self, key: &'static str) -> EventLogger {
        let policy = get_log_info();
        let log = if policy.include.contains(key) {
            let underlying: Option<Arc<Mutex<BufWriter<File>>>> = policy.path.map(|path| {
                create_dir_all(&path).unwrap();
                let suffix = format!("{}_{key}.json", self.current_context.unwrap());
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

pub fn set_task(id: Identifier, name: String) {
    LOGS.with(|local_logs| local_logs.borrow_mut().current_context = Some(id));
    get_log(NODE).log(RegistryEvent::Created(name));
    get_log(NODE).log(RegistryEvent::Created(format!(
        "{:?}",
        std::thread::current().id()
    )));
}
