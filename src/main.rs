use configs::ChainConfig;
use ethnum::{uint, int, u256, i256};
use owo_colors::OwoColorize;
use properties::Properties;
use properties::custom_functions::execute_custom_function;
use std::sync::mpsc::{Sender, Receiver};
use std::{fs, path::Path};
use serde_json::{json, Value, Number};
use std::sync::{Mutex, Arc, mpsc};

use properties::ast::*;
use properties::environment::*;
use std::str::FromStr;

use std::thread;

mod configs;
mod sockets;
mod message_formats;
mod properties;
mod utils;

fn main() {
    let mut thread_ids = vec![];
    let mut thread_names: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let dir = Path::new("config");

    let (tx, rx) :(Sender<Properties>, Receiver<Properties>) = mpsc::channel();

    let event_thread = thread::spawn(move || {
        // Event Loop
        loop {
            let property = rx.recv().unwrap();

            event_loop(property);
            
        }
    });

    thread_ids.push(event_thread);
    thread_names.lock().unwrap().push("event".to_string());

    // Run through all files in directory dir and print their paths
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        println!("{}", path.display());

        // Channel for sending 
        let sender = tx.clone();

        let thread_names_clone = Arc::clone(&thread_names);
        thread_ids.push(thread::spawn(move || {
            // Deserialize the file contents into a ChainConfig
            let contents = fs::read_to_string(path).unwrap();
            let config: ChainConfig = serde_json::from_str(&contents).unwrap();
            println!("{:?}", config);
            let mut t = thread_names_clone.lock().unwrap();
            t.push(config.get_name());
            
            config.connect(sender).unwrap();
            println!("Connected to {}", config.get_name());
        }));
    }

    for thread_id in thread_ids {
        thread_id.join().unwrap();
    }
}

fn event_loop(property: Properties) -> bool {

    // Build generic Variables from property description
    let prp = property.serialize();

    for (key, value) in prp.as_object().unwrap() {
        if value.is_string() && value.as_str().unwrap().starts_with("u256:"){
            let s = &value.as_str().unwrap()[5..];
            set_var!(key, u256::from_str(s).unwrap());
        }
        else if value.is_string() && value.as_str().unwrap().starts_with("i256:"){
            let s = &value.as_str().unwrap()[5..];
            set_var!(key, i256::from_str(s).unwrap());
        }else{
            set_var!(key, value.clone() );
        }
    }

    // Which Event?
    let event = property.occured_event.clone().unwrap();
    println!("Event: {}", event);

    let mut results: Vec<bool> = vec![];

    //println!("Dir_len {}", fs::read_dir("properties").unwrap().count());
    // Find Property Files which are triggered by the Event and the chain
    for file in fs::read_dir("properties").unwrap() {
        
        let path = file.unwrap().path();
        let def_file: Value = serde_json::from_str(fs::read_to_string(&path).unwrap().as_str()).unwrap();
        let name = path.file_name().unwrap().to_str().unwrap();
        //println!("File: {}", name);

        // Ignore events not triggered by the event or on the wrong chain
        if property.src_chain.clone().unwrap().to_lowercase() == "ethereum" {
            let ev = def_file.get("event").unwrap().as_str().unwrap();
            if ev != event {
                let hashed_event = utils::get_ethereum_topic_ids(ev);
                if hashed_event != event{
                    //println!("hashed_event: {}, event: {}", hashed_event, event);
                    continue;
                }
            }
        } else {   
            if def_file.get("event").unwrap().as_str().unwrap() != event 
            || def_file.get("chain_name").unwrap().as_str().unwrap().to_lowercase() != property.src_chain.clone().unwrap().to_lowercase() {
                //println!("Continuing...");
                continue;
            }
        }
        // Following files are all correct    
        
        // Execute Custom Functions and get Variables
        let vars = execute_custom_function(&def_file).unwrap();

        for (key, value) in vars {
            if value.is_string() && value.as_str().unwrap().starts_with("u256:"){
                let s = &value.as_str().unwrap()[5..];
                set_var!(key, u256::from_str(s).unwrap());
            }
            else if value.is_string() && value.as_str().unwrap().starts_with("i256:"){
                let s = &value.as_str().unwrap()[5..];
                set_var!(key, i256::from_str(s).unwrap());
            }else{
                set_var!(key, value);
            }
        }

        // parse pattern into AST
        let pattern = def_file.get("pattern").unwrap().as_array().unwrap();
        // Transform all patterns into one large string
        let processed_pattern = pattern.iter().map(|p| p.as_str().unwrap().to_string()).collect::<Vec<String>>().join(" && ");
        println!("Pattern: {}", processed_pattern);

        let (ast, root) = build_ast!(processed_pattern);

        // Evaluate AST
        let val = root.evaluate().unwrap();
        let ret: bool = val.get_value().parse().unwrap();

        // Save result
        if ret {
            println!("{} transaction: {} From: {}", "Allow".green(), property.transaction_hash.clone().unwrap(), name.yellow());
            results.push(true);
        } else {
            println!("{} transaction: {} From: {}", "Deny".red(), property.transaction_hash.clone().unwrap(), name.yellow());
            results.push(false);
        }
    }

    // Check all results and only allow when all are true
    if results.iter().all(|x| *x) {
        println!("{} transaction: {}", "Allow".green(), property.transaction_hash.clone().unwrap());
        true
    }else {
        println!("{} transaction: {}", "Deny".red(), property.transaction_hash.clone().unwrap());
        false
    }

}