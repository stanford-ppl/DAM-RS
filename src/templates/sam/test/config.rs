use serde_derive::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Data {
    pub sam_config: Config,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub sam_path: String,
}

#[cfg(test)]
mod tests {

    use super::Config;

    #[test]
    fn get_path() {
        let config: Config = toml::from_str("sam_path = '$HOME'").unwrap();
        dbg!(config);
        // let filename = "/home/rubensl/sam_config.toml";
        // let contents = match fs::read_to_string(filename) {
        //     Ok(c) => c,
        //     Err(_) => {
        //         panic!("File not found");
        //     }
        // };

        // dbg!(&contents);

        // let data: Data = match toml::from_str(&contents) {
        //     // If successful, return data as `Data` struct.
        //     // `d` is a local variable.
        //     Ok(d) => d,
        //     // Handle the `error` case.
        //     Err(_) => {
        //         // Write `msg` to `stderr`.
        //         eprintln!("Unable to load data from `{}`", filename);
        //         return;
        //         // Exit the program with exit code `1`.
        //     }
        // };

        // dbg!(data);
    }
}
