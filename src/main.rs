use configs::ChainConfig;
use ethnum::{u256, i256};
use owo_colors::OwoColorize;
use properties::Properties;
use properties::custom_functions::execute_custom_function;
use sockets::event_socket::{Event, BlockingQueue, Allowance};
use std::collections::HashMap;
use std::io::{Write, Read};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{Sender, Receiver};
use std::{fs, path::Path};
use serde_json::Value;
use std::sync::{Mutex, Arc, mpsc};

use properties::ast::*;
use properties::environment::*;
use std::str::FromStr;

use std::thread::{self, JoinHandle};

use clap::Parser;

mod configs;
mod sockets;
mod message_formats;
mod properties;
mod utils;

/// Arguments to the program
#[derive(Parser, Debug)]
#[command(name = "Brigade")]
#[command(author = "Pascal Winkler <pascal.winkler@uni-due.de>")]
#[command(version = "0.1.0")]
#[command(about = "Brigade secures Cross Chain Bridges", long_about = None)]
struct Args {
    /// Endpoint at which the TCP Port is opened. Default: 127.0.0.1:8080
    #[arg(short, long)]
    endpoint: Option<String>,
}

fn main() {


    // Argument parsing
    let args = Args::parse();
    let mut ip_addr = "127.0.0.1:8080".to_string();

    if let Some(endpoint) = args.endpoint {
        ip_addr = endpoint;
    }

    println!("Connecting at {}", ip_addr);

    // Start threads for Chains and Events
    let mut thread_ids = vec![];
    let mut thread_names: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let dir = Path::new("config");

    // Build Message Channels
    let (tx, rx) :(Sender<Properties>, Receiver<Properties>) = mpsc::channel();

    let event_thread = thread::spawn(move || {
        // Setup the Event Socket
        let event_queue: Arc<BlockingQueue<Event>> = Arc::new(BlockingQueue::new());
        let (handle1, handle2) = setup_event_ws(ip_addr, event_queue.clone()).unwrap();

        // Event Loop
        loop {
            let property = rx.recv().unwrap();

            event_loop(property, event_queue.clone());
            
        }

        handle1.join().unwrap();
        handle2.join().unwrap();
    });

    thread_ids.push(event_thread);
    thread_names.lock().unwrap().push("event".to_string());

    // Setup persistent keystore
    set_var!("keystore", VarValues::Array(vec![]));

    // Setup persistent Hashmap
    set_var!("map", VarValues::Map(HashMap::new()));

    // Run through all files in directory dir and print their paths
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        println!("{}", path.display());

        // Channel for sending 
        let sender = tx.clone();

        let thread_names_clone = Arc::clone(&thread_names);
        thread_ids.push(thread::spawn(move || {
            // Deserialize the file contents into a ChainConfig
            let contents = fs::read_to_string(path.clone()).unwrap();
            let config: ChainConfig = serde_json::from_str(&contents).unwrap();
            let mut t = thread_names_clone.lock().unwrap();
            t.push(config.get_name());
            
            let contract_name = path.file_name().unwrap().to_str().unwrap().split('_').collect::<Vec<&str>>()[0];

            let cn = contract_name.to_string() + "_contract";
            set_var!(cn, config.get_contract_address());

            config.connect(sender).unwrap();
            println!("Connected to {}", config.get_name());
        }));
    }

    for thread_id in thread_ids {
        thread_id.join().unwrap();
    }
}

fn event_loop(property: Properties, event_queue: Arc<BlockingQueue<Event>>) -> bool {
    // Build generic Variables from property description
    let prp = property.serialize();

    for (key, value) in prp.as_object().unwrap() {
        if key.to_string() == "block_number"{
            let bn = property.src_chain.clone().unwrap() + "_" + key;
            let v = &value.as_str().unwrap()[5..];
            println!("{}: {}", bn, v);
            set_var!(bn, u256::from_str(v).unwrap());
            continue;
        }
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

    println!("Variables: {:?}", list_variables(get_variable_map_instance()));

    // Which Event?
    let event = property.occured_event.clone().unwrap();
    println!("Event: {}", event.blue());
    println!("Transaction Hash: {}", property.transaction_hash.clone().unwrap().blue());
    println!("Chain: {}", property.src_chain.clone().unwrap().blue());

    // Results of the separate files
    let mut results: Vec<bool> = vec![];
    // Which files were relevant
    let mut checked_vec: Vec<String> = vec![];
    // Which file was failed
    let mut fail_reason: Vec<String> = vec![];
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
        // Push onto checked Vec
        checked_vec.push(name.clone().to_owned());
        
        // Execute Custom Functions and get Variables
        let _ = execute_custom_function(&def_file).unwrap();

        // for (key, value) in vars {
        //     if value.is_string() && value.as_str().unwrap().starts_with("u256:"){
        //         let s = &value.as_str().unwrap()[5..];
        //         set_var!(key, u256::from_str(s).unwrap());
        //     }
        //     else if value.is_string() && value.as_str().unwrap().starts_with("i256:"){
        //         let s = &value.as_str().unwrap()[5..];
        //         set_var!(key, i256::from_str(s).unwrap());
        //     }else{
        //         set_var!(key, value);
        //     }
        // }

        // parse pattern into AST
        let pattern = def_file.get("pattern").unwrap().as_array().unwrap();
        // Transform all patterns into one large string
        let processed_pattern = pattern.iter().map(|p| p.as_str().unwrap().to_string()).collect::<Vec<String>>().join(" && ");
        // println!("Pattern: {}", processed_pattern);

        let root = build_ast_root(processed_pattern.as_str()).unwrap();
        root.print("");

        // Evaluate AST
        let val = root.evaluate();
        match val {
            Ok(v) => {
                let ret: bool = v.get_value().parse().unwrap();     
                        // Save result
                if ret {
                    println!("{} transaction: {} From: {}", "Allow".green(), property.transaction_hash.clone().unwrap(), name.yellow());
                    results.push(true);
                } else {
                    println!("{} transaction: {} From: {}", "Deny".red(), property.transaction_hash.clone().unwrap(), name.yellow());
                    fail_reason.push(name.clone().to_string());
                    results.push(false);
                }   
            }
            Err(e) => {
                println!("{} transaction: {} From: {}", "Deny".red(), property.transaction_hash.clone().unwrap(), name.yellow());
                println!("Error: {}", e);
                fail_reason.push(name.clone().to_string());
                results.push(false);
            }
        }
        


    }
    let is_allowed: Allowance = if results.iter().all(|x| *x) { 
        Allowance::Allow 
    } else {
        Allowance::Deny(fail_reason.clone())
    };
    let event = Event {
        result: is_allowed,
        checked: checked_vec,
        chain: property.src_chain.clone().unwrap(),
        transaction_hash: property.transaction_hash.clone().unwrap(),
    };

    event_queue.push(event);

    println!("Variables: {:?}", get_variable_map_instance());

    // Clear all non persistent variables
    let map = get_variable_map_instance();
    map.retain(|k, _| *k == "keystore" || *k == "map" || k.contains("_contract"));

    println!("Variables: {:?}", get_variable_map_instance());

    // Check all results and only allow when all are true
    if results.iter().all(|x| *x) {
        println!("{} transaction: {}", "Allow".green(), property.transaction_hash.clone().unwrap());
        true
    }else {
        println!("{} transaction: {}", "Deny".red(), property.transaction_hash.clone().unwrap());
        false
    }

}

// Setup a websocket thread acting as a broadcaster for events
fn setup_event_ws(addr: String, event_queue: Arc<BlockingQueue<Event>>) -> Result<(JoinHandle<()>, JoinHandle<()>), String> {
    
    // Build TCP Endpoint
    let listener = TcpListener::bind(addr).expect("Failed to bind Address");
    let connections: Arc<Mutex<Vec<TcpStream>>> = Arc::new(Mutex::new(Vec::new()));
    let connections_clone = Arc::clone(&connections);

    // Get Connections
    let connection_handler = thread::spawn(move ||{
        for stream in listener.incoming() {
            let mut connections = connections_clone.lock().unwrap();
            println!("New connection: {}", stream.as_ref().unwrap().peer_addr().unwrap());
            connections.push(stream.unwrap());
        }
    });
    //let event_queue_clone: Arc<BlockingQueue<Event>> = Arc::clone(&event_queue);

    // await events
    let connections_clone2 = Arc::clone(&connections);
    let event_handler = thread::spawn(move || {
        loop {
            let event = event_queue.pop();
            println!("{:?}", event);
            connections_clone2.lock().unwrap().iter().for_each(|mut x| {
                match x.write_all(&serde_json::to_vec(&event).unwrap()) {
                    Ok(_) => {},
                    Err(e) => {println!("Error {}: {}", x.peer_addr().unwrap(), e);}
                }
            })
        }
    });
    // broadcast events

    Ok((connection_handler, event_handler))
}

#[test]
fn test_event_broadcast() {
    let event_queue: Arc<BlockingQueue<Event>> = Arc::new(BlockingQueue::new());
    let (handle1, handle2) = setup_event_ws("127.0.0.1:8080".to_string(), event_queue.clone()).unwrap();

    let remote_thread = thread::spawn(move || {
        let mut remote = TcpStream::connect("127.0.0.1:8080").unwrap();
        let mut buffer: [u8; 128] = [0; 128];
        remote.read(&mut buffer).unwrap();
        println!("Received: {}", String::from_utf8_lossy(&buffer));
    });

    let mut wait_interval = 0;
    while wait_interval < 100000 {
        wait_interval += 1;
    }

    let event: Event = Event {
        result: Allowance::Allow,
        checked: vec!["definition1".to_string()],
        chain: "ethereum".to_string(),
        transaction_hash: "123".to_string(),
    };
    event_queue.clone().push(event);

    handle1.join().unwrap();
    handle2.join().unwrap();
    remote_thread.join().unwrap();

}

#[test]
fn test_receiving(){
    let remote_thread = thread::spawn(move || {
        let mut remote = TcpStream::connect("127.0.0.1:8080").unwrap();
        let mut buffer: [u8; 128] = [0; 128];
        remote.read(&mut buffer).unwrap();
        println!("Received: {}", String::from_utf8_lossy(&buffer));
    });

    let remote_thread2 = thread::spawn(move || {
        let mut remote = TcpStream::connect("127.0.0.1:8080").unwrap();
        let mut buffer: [u8; 128] = [0; 128];
        remote.read(&mut buffer).unwrap();
        println!("Received: {}", String::from_utf8_lossy(&buffer));
    });

    let remote_thread3 = thread::spawn(move || {
        let mut remote = TcpStream::connect("127.0.0.1:8080").unwrap();
        let mut buffer: [u8; 128] = [0; 128];
        remote.read(&mut buffer).unwrap();
        println!("Received: {}", String::from_utf8_lossy(&buffer));
    });

    remote_thread.join().unwrap();
    remote_thread2.join().unwrap();
    remote_thread3.join().unwrap();

}
