use configs::ChainConfig;
use std::{fs, path::Path};
use serde_json::{json, Value};

use std::thread;

mod configs;
mod socket;
mod message_formats;
mod properties;
mod utils;

fn main() {
    let mut thread_ids = vec![];
    let dir = Path::new("config");

    // Run through all files in directory dir and print their paths
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        println!("{}", path.display());

        thread_ids.push(thread::spawn( || {
            // Deserialize the file contents into a ChainConfig
            let contents = fs::read_to_string(path).unwrap();
            let config: ChainConfig = serde_json::from_str(&contents).unwrap();
            println!("{:?}", config);

            config.connect().unwrap();
            println!("Connected to {}", config.get_name());
        }));
    }

    for thread_id in thread_ids {
        thread_id.join().unwrap();
    }
}
