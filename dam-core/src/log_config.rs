use std::{collections::HashSet, path::PathBuf, sync::OnceLock};

use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use crate::{config::get_config, metric::METRICS};

#[derive(Clone, Debug)]
pub struct LogInfo {
    pub path: Option<PathBuf>,
    pub include: HashSet<String>,
    pub eager_flush: bool,
}

static LOG_INFO: OnceLock<LogInfo> = OnceLock::new();
pub fn get_log_info() -> LogInfo {
    LOG_INFO
        .get_or_init(|| get_config().log_config.clone().into())
        .clone()
}

// The "ALL" string is special in that it either includes or excludes everything.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LogConfig {
    path: Option<String>,
    log_options: Option<Table>,
    eager_flush: Option<bool>,
}

impl LogConfig {
    pub fn validate(&self) -> bool {
        if let Some(options) = &self.log_options {
            let all_bools = options.iter().all(|(_k, v)| {
                // Check if value is a boolean
                if let Value::Boolean(_) = v {
                    true
                } else {
                    false
                }
            });
            if !all_bools {
                return false;
            }

            // Check if all of the keys are in METRICS
            return options.iter().all(|(k, _)| METRICS.contains(&k.as_str()));
        }
        return true;
    }

    pub fn update(&mut self, other: LogConfig) {
        // If we already have a path, then we concatenate.
        let new_path = match (&self.path, &other.path) {
            (None, None) => None,
            (None, Some(path)) => Some(path.clone()),
            (Some(path), None) => Some(path.clone()),
            (Some(a), Some(b)) => {
                let mut pb = PathBuf::new();
                pb.push(a);
                pb.push(b);
                pb.to_str().map(|x| x.to_string())
            }
        };
        self.path = new_path;
        match (&mut self.log_options, &other.log_options) {
            (None, None) => {}
            (None, Some(options)) => self.log_options = Some(options.clone()),
            (Some(_), None) => {}
            (Some(base), Some(new)) => base.extend(new.clone().into_iter()),
        }
        if other.eager_flush.is_some() {
            self.eager_flush = other.eager_flush;
        }
    }
}

impl From<LogConfig> for LogInfo {
    fn from(value: LogConfig) -> Self {
        let all_options = value
            .log_options
            .into_iter()
            .flat_map(|opts| opts.into_iter());
        let filtered =
            all_options.filter(|(_, v)| if let Value::Boolean(x) = v { *x } else { false });

        let include = HashSet::from_iter(filtered.map(|(k, _)| k));

        Self {
            path: value.path.map(PathBuf::from),
            include,
            eager_flush: value.eager_flush.unwrap_or(false),
        }
    }
}
