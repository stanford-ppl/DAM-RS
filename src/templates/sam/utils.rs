use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn set_tensor_path() {
    env::set_var("FROSTT_FORMATTED_PATH", "/home/rubensl/data");
}

fn read_inputs<T>(file_path: &str) -> Vec<T>
where
    // Vec<T>: FromIterator<u32>,
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
        let b_dirname = Path::new(&dirname)
            .join("B_linear")
            .join("tensor3_dropout")
            .join("tensor_B_mode_0_crd");
        // let b_dirname = [dirname, "B_linear".to_owned()].join("/");
        dbg!(b_dirname);

        // let st = b_dirname.to_str().unwrap().clone();

        // let v = read_inputs::<u32>(&st);
    }
}
