use std::{
    collections::{hash_map::DefaultHasher, HashMap, HashSet},
    fs::File,
    hash::{Hash, Hasher},
    io::BufWriter,
    path::PathBuf,
    sync::{OnceLock, RwLock, RwLockWriteGuard},
};

use super::identifier::Identifier;

type CPTType = HashMap<Identifier, Identifier>;

#[derive(Default, Debug)]
pub struct LogGraph {
    // Maps children to their parents
    child_parent_tree: RwLock<CPTType>,

    // Maps threads to file paths
    logging_paths: RwLock<HashMap<Identifier, PathBuf>>,
    // All identifiers
    all_identifiers: RwLock<HashSet<Identifier>>,
}

impl LogGraph {
    pub fn dump(&self) {
        let cpt = self.child_parent_tree.read().unwrap();
        for (k, v) in cpt.iter() {
            println!("Child({k:?}) -> Parent({v:?})");
        }

        let ids = self.all_identifiers.read().unwrap();
        for id in ids.iter() {
            if !cpt.contains_key(id) {
                println!("Orphan ID: {id:?}");
            }
        }
    }

    pub fn add_id(&self, id: Identifier) {
        self.all_identifiers.write().unwrap().insert(id);
    }
}

pub struct LogGraphHandle<'a> {
    self_id: Identifier,
    under: RwLockWriteGuard<'a, CPTType>,
}

impl<'a> LogGraphHandle<'a> {
    pub fn add_child(&mut self, child: Identifier) {
        self.under.insert(child, self.self_id);
    }
}

pub fn get_graph() -> &'static LogGraph {
    GRAPH.get_or_init(Default::default)
}

static GRAPH: OnceLock<LogGraph> = OnceLock::new();

pub fn register_handle(self_id: Identifier) -> LogGraphHandle<'static> {
    let g = GRAPH.get_or_init(Default::default);
    LogGraphHandle {
        self_id,
        under: g.child_parent_tree.write().unwrap(),
    }
}

pub fn get_log_path(self_id: Identifier) -> Option<PathBuf> {
    let g = GRAPH.get_or_init(Default::default);
    let mut cur_thread = self_id;
    loop {
        match g.logging_paths.read().unwrap().get(&cur_thread) {
            // One of our parents had a registered path!
            Some(path) => return Some(path.clone()),
            None => {
                let tree_handle = g.child_parent_tree.read().unwrap();
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
    let mut paths = g.logging_paths.write().unwrap();

    // It's fine to fail on the remove_dir_all, since the path might not exist.
    let _ = std::fs::remove_dir_all(path.clone());

    // This one isn't allowed to fail since we're going to be writing there.
    std::fs::create_dir_all(path.clone()).unwrap();

    paths.insert(id, path);
}

pub fn get_file(
    self_id: Identifier,
    self_name: Option<&str>,
    suffix: &str,
    ext: &str,
) -> Option<BufWriter<File>> {
    get_filename(self_id, self_name, suffix, ext)
        .map(|fname| BufWriter::new(File::create(fname).unwrap()))
}

pub fn get_filename(
    self_id: Identifier,
    self_name: Option<&str>,
    suffix: &str,
    ext: &str,
) -> Option<PathBuf> {
    match get_log_path(self_id) {
        Some(dir) => {
            // TODO: Once threadId::as_u64() is stabilized, use that instead of hashing.
            let mut hasher = DefaultHasher::new();
            self_id.hash(&mut hasher);

            let name_suffix = match self_name {
                Some(name) => format!("-{name}"),
                None => "".to_string(),
            };

            let fname = format!("{}{}-{suffix}.{ext}", hasher.finish(), &name_suffix,);
            let full_path = dir.join(fname);
            Some(full_path)
        }
        None => None,
    }
}
