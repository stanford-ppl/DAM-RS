use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{env, thread};

fn set_tensor_path() {
    env::set_var("FROSTT_FORMATTED_PATH", "/home/rubensl/Documents/data");
}

// pub fn read_inputs<T>(file_path: &PathBuf) -> Result<Vec<T>, std::io::Error>
pub fn read_inputs<T>(file_path: &PathBuf) -> Vec<T>
where
    T: std::str::FromStr,
{
    let file = File::open(file_path).expect("file wasn't found.");
    let reader = BufReader::new(file);

    let v: Vec<T> = reader
        .lines()
        .flatten() // gets rid of Err from lines
        .flat_map(|line| line.parse::<T>()) // ignores Err variant from Result of str.parse
        .collect();
    v
}

// pub struct CSF_
fn process_file<T: std::str::FromStr>(file_path: &PathBuf, shared_map: Arc<Mutex<Vec<Vec<T>>>>) {
    let mut map = shared_map.lock().unwrap();
    // map.insert(*file_path, vector);
    let vector = read_inputs(file_path);
    map.push(vector);
}

pub fn par_read_inputs<T>(base_path: &PathBuf, files: &Vec<String>) -> Vec<Vec<T>>
// ) -> HashMap<PathBuf, Vec<Vec<T>>>
where
    T: std::str::FromStr + std::marker::Send,
{
    // let shared_map: Arc<Mutex<HashMap<PathBuf, Vec<T>>>> = Arc::new(Mutex::new(HashMap::new()));
    let shared_vec: Arc<Mutex<Vec<Vec<T>>>> = Arc::new(Mutex::new(Vec::new()));
    // let shared_map: Arc<HashMap<PathBuf, Vec<T>>> = Arc::new(HashMap::new());

    thread::scope(|td| {
        for file_name in files {
            td.spawn(|| {
                process_file::<T>(
                    &(Path::new(base_path).join(file_name.clone())),
                    shared_vec.clone(),
                );
            });
        }
    });

    Arc::try_unwrap(shared_vec)
        .ok()
        .unwrap()
        .into_inner()
        .unwrap()
}

#[cfg(test)]
mod tests {
    use std::{env, path::Path};

    use super::read_inputs;
    use super::set_tensor_path;

    #[test]
    fn test() {
        set_tensor_path();
        let frostt = env::var("FROSTT_FORMATTED_PATH").unwrap();
        dbg!(frostt);
    }

    #[test]
    fn read_test() {
        set_tensor_path();
        let dirname = env::var("FROSTT_FORMATTED_PATH").unwrap();
        let binding = Path::new(&dirname)
            .join("B_linear")
            .join("tensor3_dropout")
            .join("tensor_B_mode_0_crd");
        // let b_dirname = binding.to_str().unwrap();

        let v = read_inputs::<u32>(&binding);
        dbg!(v);
    }
}
