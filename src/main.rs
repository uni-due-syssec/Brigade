use configs::ChainConfig;
use ethnum::{ i256, u256 };
use owo_colors::colors::css::DarkCyan;
use owo_colors::OwoColorize;
use properties::custom_functions::execute_custom_function;
use properties::Properties;
use serde_json::Value;
use sockets::event_socket::{ Allowance, BlockingQueue, Event };
use std::cmp::min;
use std::collections::{ HashMap, HashSet };
use std::fs::{ File, OpenOptions };
use std::io::{ self, ErrorKind, Read, Write };
use std::net::{ TcpListener, TcpStream };
use std::path::PathBuf;
use std::sync::atomic::{ self, AtomicBool, AtomicU64 };
use std::sync::mpsc::{ Receiver, Sender };
use std::sync::{ mpsc, Arc, Mutex };
use std::time::{ Duration, Instant };
use std::{ fs, path::Path };

use properties::ast::*;
use properties::environment::*;
use std::str::FromStr;

use std::thread::{ self, sleep, JoinHandle };

use clap::Parser;

use chrono::{ DateTime, Datelike, Local, Timelike };

use crate::configs::BridgeConfig;
use crate::configs::connection::ConnectionConfig;
use crate::inference::ModelFeature;
use crate::properties::talon::TalonFile;
use crate::sockets::replay_ethereum_socket;
use crate::utils::{ get_startup_time, Evaluation };

mod configs;
mod inference;
mod message_formats;
mod properties;
mod sockets;
mod utils;

static LOG_TIMESTAMPS: AtomicBool = AtomicBool::new(false);
static LAST_ID: AtomicU64 = AtomicU64::new(0);
static IS_TRAINED: AtomicBool = AtomicBool::new(false);
static TRAINED_ON: AtomicU64 = AtomicU64::new(500);
static ALREADY_TRAINED: AtomicU64 = AtomicU64::new(0);
const FEATURE_VEC_LENGTH: usize = 10;

/// Arguments to the program
#[derive(Parser, Debug)]
#[command(name = "Brigade")]
#[command(author = "Anonymous <anonymous>")]
#[command(version = "0.1.0")]
#[command(about = "Brigade secures Cross Chain Bridges", long_about = None)]
struct Args {
    /// Endpoint at which the TCP Port is opened. Default: 127.0.0.1:8080
    #[arg(long)]
    endpoint: Option<String>,
    /// Use predefined variables from a json file.
    /// The json file should contain an array containing the patterns for creation of variables
    /// Example:
    /// [
    ///     "keystore.push(0xaabbccddeeff0011)",
    ///     "keystore.push(0xaabbccddeeff0012)"
    /// ]
    #[arg(short, long)]
    predefined_variables: Option<PathBuf>,
    /// Log Timestamps for evaluation
    #[arg(short, long)]
    log_timestamps: bool,
    /// Run in replay mode
    #[arg(short, long)]
    replay: bool,
    // /// Starting block for replay
    // #[arg(short, long)]
    // start_block: Option<u256>,
    // /// End block for replay (inclusive)
    // #[arg(long)]
    // end_block: Option<u256>,
    /// Path to replay config
    #[arg(long)]
    replay_config: Option<PathBuf>,
}

fn main() {
    // Argument parsing
    let args = Args::parse();
    let mut ip_addr = "127.0.0.1:8080".to_string();

    if let Some(endpoint) = args.endpoint {
        ip_addr = endpoint;
    }

    if args.log_timestamps {
        LOG_TIMESTAMPS.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    // Log starting point
    let current_datetime: DateTime<Local> = Local::now();
    let hour = current_datetime.hour();
    let minute = current_datetime.minute();
    let second = current_datetime.second();
    let date_time = format!("{:02}:{:02}:{:02}", hour, minute, second);
    let startup_time = get_startup_time();

    // Setup persistent keystore
    set_var!("keystore", VarValues::Array(vec![]));

    // Setup persistent Hashmap
    set_var!("map", VarValues::Map(HashMap::new()));

    // Setup predefined variables
    if let Some(predefined_variables) = args.predefined_variables {
        let var_file = predefined_variables.as_path();
        let contents = fs::read_to_string(var_file).unwrap();
        let values: Value = serde_json::from_str(&contents).unwrap();
        match values.as_array() {
            Some(content) => {
                for var in content.iter() {
                    match build_ast_root(var.as_str().unwrap()) {
                        Ok(root) => {
                            root.print("");
                            match root.evaluate() {
                                Ok(val) => {
                                    println!("{}: {}", var, val.get_value());
                                }
                                Err(e) => {
                                    println!("Error when parsing {}", var);
                                    eprintln!("Error: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            println!("Can't create AST from {}", var);
                            eprintln!("Error: {}", e);
                        }
                    }
                }
            }
            None => {
                println!("Resuming without variables");
                eprintln!("Error: Wrong file format");
            }
        }
        println!("Variables: {:?}", get_variable_map_instance());
    }

    println!("Connecting at {}", ip_addr);

    // Start threads for Chains and Events
    let mut thread_ids = vec![];
    let dir = Path::new("config");

    // Build Message Channels
    let (tx, rx): (Sender<Properties>, Receiver<Properties>) = mpsc::channel();

    let event_thread = thread::spawn(move || {
        // Setup the Event Socket
        let event_queue: Arc<BlockingQueue<Event>> = Arc::new(BlockingQueue::new());
        let (handle1, handle2) = setup_event_ws(ip_addr, event_queue.clone()).unwrap();

        // Event Loop
        loop {
            let property = rx.recv().unwrap();
            event_loop(property.clone(), event_queue.clone());
        }

        handle1.join().unwrap();
        handle2.join().unwrap();
    });

    // thread_ids.push(event_thread);

    if !args.replay {
        // Run through all files in directory dir and print their paths
        for entry in fs::read_dir(dir).unwrap() {
            sleep(Duration::from_millis(100));
            let path = entry.unwrap().path();
            println!("{}", path.display());

            // Skip auxilary config files
            if path.file_name().unwrap() == "replay_config.json" {
                println!("Skipping Replay");
                continue;
            }

            if path.file_name().unwrap() == "connections.json" {
                println!("Skipping Connections");
                continue;
            }

            // Channel for sending
            let sender = tx.clone();

            // let thread_names_clone = Arc::clone(&thread_names);
            // Deserialize the file contents into a ChainConfig
            let contents = fs::read_to_string(&path).unwrap();
            let bd: BridgeConfig = serde_json::from_str(&contents).unwrap();

            for config in bd.contracts {
                let sender_clone = sender.clone();

                let contract_name =
                    config.contract_name.clone().unwrap_or("contract".to_string()) +
                    "_" +
                    &config.name;
                set_var!(contract_name, config.get_contract_address());

                // thread_names_clone.lock().unwrap().push(contract_name.to_string());

                // TODO: if replay then connect_replay instead of connect
                // Instead of connecting, we replay the blocks by sending the transaction to replay a block
                match config.connect(sender_clone) {
                    Ok(_) => println!("Connected to {}", config.get_name()),
                    Err(_) => println!("Failed to connect to {}", config.get_name()),
                }
            }
        }
    } else {
        // Replay mode:

        // Setup blockrange
        // let block_start = args
        //     .start_block
        //     .expect("Start block must be provided for replaying transactions");
        // let block_end = args
        //     .end_block
        //     .expect("End block must be provided for replaying transactions");
        // let mut block_number = block_start;

        let config: replay_ethereum_socket::ReplayConfig = serde_json
            ::from_str(
                fs
                    ::read_to_string(
                        args.replay_config
                            .expect(
                                "Replay config must be provided. See ReplayConfig in replay_ethereum_socket.rs"
                            )
                            .to_str()
                            .unwrap()
                    )
                    .unwrap()
                    .as_str()
            )
            .unwrap();
        let start = u64
            ::from_str_radix(config.starting_block.trim_start_matches("0x"), 16)
            .expect("invalid start hex");
        let end = if &config.ending_block == "latest" {
            unimplemented!("latest not implemented");
        } else {
            u64::from_str_radix(&config.ending_block.trim_start_matches("0x"), 16).expect(
                "invalid end hex"
            )
        };
        let step_len = config.page_length.unwrap_or(10000);
        let step = if config.paging.unwrap_or(false) { step_len } else { 10_000 };

        for chain in config.chains {
            let tx_clone = tx.clone();
            thread_ids.push(
                thread::spawn(move || {
                    let connections = ConnectionConfig::from_file("config/connections.json");
                    let chain_connection = connections.connections
                        .iter()
                        .find(|x| x.name == chain.name)
                        .unwrap();

                    // call the replay function and then invoke the replay handler and send the resulting properties via tx to rx
                    let replayer = replay_ethereum_socket::ReplayEthereumSocketHandler {
                        chain_name: chain.name.to_string(),
                        config: chain,
                        rpc_url: chain_connection.rpc_url.to_string(),
                    };

                    // let txs = replayer.get_all_logs().unwrap();

                    for i in (start..=end).step_by(step as usize) {
                        let end_block = min(end, i+step);
                        let txs = replayer.get_logs(format!("0x{:x}", i), format!("0x{:x}", end_block));
                        match txs {
                            Ok(txs) => {
                                println!("Length of txs: {}", txs.len());
                                // Send to tx
                                for t in txs {
                                    tx_clone.send(t).unwrap();
                                }
                            }
                            Err(e) => eprintln!("Error: {}", e),
                        }
                    }
                })
            );
        }

        // TODO: Terminate the program gracefully
    }

    // sleep(Duration::from_secs(5));

    // println!(
    //     "Length of Websockets: {}",
    //     configs::connection::get_established_connections().len()
    // );

    for t in thread_ids {
        t.join().unwrap();
    }

    // Wait for user termination
    let mut input = String::new();
    let stdin = io::stdin();
    print!("Press 'q' to terminate the program...");
    loop {
        stdin.read_line(&mut input).expect("Failed to read line");
        if input.trim() == "q" {
            break;
        }
        input.clear();
    }

    drop(event_thread);
}

fn event_loop(property: Properties, event_queue: Arc<BlockingQueue<Event>>) -> bool {
    let mut ev: Evaluation = Evaluation::default();
    ev.id = LAST_ID.load(atomic::Ordering::Relaxed);
    LAST_ID.store(ev.id + 1, atomic::Ordering::Relaxed);
    let now = Instant::now();

    // Build generic Variables from property description
    let prp = property.serialize();

    for (key, value) in prp.as_object().unwrap() {
        // println!("Adding {} to Map {:p}", key, get_variable_map_instance());
        if key.to_string() == "block_number" {
            let bn = property.src_chain.clone().unwrap() + "_" + key;
            let v = &value.as_str().unwrap()[5..];
            // println!("{}: {}", bn, v);
            set_var!(bn, u256::from_str(v).unwrap());
            continue;
        }
        if value.is_string() && value.as_str().unwrap().starts_with("u256:") {
            let s = &value.as_str().unwrap()[5..];
            set_var!(key, u256::from_str(s).unwrap());
        } else if value.is_string() && value.as_str().unwrap().starts_with("i256:") {
            let s = &value.as_str().unwrap()[5..];
            set_var!(key, i256::from_str(s).unwrap());
        } else {
            set_var!(key, value.clone());
        }

        if key.to_string() == "event_data" {
            let event_data = value.as_str().unwrap();
            set_var!("event_data", event_data);
        }
    }

    // Print the variables before the properties are processed
    print_variables(&get_variable_map_instance());

    // Which Event?
    let event = property.occured_event.clone().unwrap();
    println!("Event: {}", event.blue());
    println!("Transaction Hash: {}", property.transaction_hash.clone().unwrap().blue());
    println!("Chain: {}", property.src_chain.clone().unwrap().blue());

    ev.event_type = event.clone();

    // Results of the separate files
    let mut results: Vec<bool> = vec![];
    // Which files were relevant
    let mut checked_vec: Vec<String> = vec![];
    // Which file was failed
    let mut fail_reason: Vec<String> = vec![];

    // Process the properties
    process_json_properties(property.clone(), &mut results, &mut checked_vec, &mut fail_reason);

    // process_talon_code(property.clone(), &mut results, &mut fail_reason);

    ev.duration = now.elapsed().as_millis();

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

    // Clear all non persistent variables
    let map = get_variable_map_instance();
    /*
     * This removes all variables generated during the call of the property.
     * Some Variables however are needed to be kept for future calls of the property.
     * They can be defined here.
     */
    map.retain(|k, _| (*k == "keystore" || *k == "map" || k.contains("_contract")));

    print_variables(&map);

    // pub contract_address: String,
    let call = format!(
        "call({}, eth_getTransactionByHash, [{}]).get(result)",
        property.src_chain.unwrap_or("ethereum".to_string()),
        property.transaction_hash.clone().unwrap()
    );
    let root = build_ast_root(call.as_str()).unwrap();
    root.print("");
    let val = root.evaluate().unwrap();
    println!("{}", val.get_value());

    ev.contract_address = val
        .get_map()
        .get("to")
        .expect("Failed to get 'to' from message")
        .get_value();
    // pub msg_sender: String,
    ev.msg_sender = val
        .get_map()
        .get("from")
        .expect("Failed to get 'from' from message")
        .get_value();
    // pub block_number: String,
    ev.block_number = val
        .get_map()
        .get("blockNumber")
        .expect("Failed to get 'blockNumber' from message")
        .get_value();
    // pub msg_value: u256,
    let v = val.get_map().get("value").expect("Failed to get 'value' from message").get_value();
    ev.msg_value = u256::from_str_hex(&v).unwrap();

    if fail_reason.len() > 0 {
        // pub pattern_type: String,
        ev.pattern_type = fail_reason.join(" AND ").to_string();
    } else {
        ev.pattern_type = "allowed".to_string();
    }

    log_evaluation(ev);

    // Check all results and only allow when all are true
    if results.iter().all(|x| *x) {
        println!("{} transaction: {}", "Allow".green(), property.transaction_hash.clone().unwrap());
        true
    } else {
        println!("{} transaction: {}", "Deny".red(), property.transaction_hash.clone().unwrap());
        false
    }
}

fn process_json_properties(
    property: Properties,
    results: &mut Vec<bool>,
    checked_vec: &mut Vec<String>,
    fail_reason: &mut Vec<String>
) -> bool {
    let event = property.occured_event.clone().unwrap();
    // println!("Dir_len {}", fs::read_dir("properties").unwrap().count());
    // Find Property Files which are triggered by the Event and the chain
    for file in fs::read_dir("properties").unwrap() {
        let path = file.unwrap().path();
        let def_file: Value = serde_json
            ::from_str(fs::read_to_string(&path).unwrap().as_str())
            .unwrap();
        let name = path.file_name().unwrap().to_str().unwrap();
        // println!("File: {:?}", serde_json::to_string_pretty(&def_file).unwrap());

        // Ignore events not triggered by the event or on the wrong chain
        if property.src_chain.clone().unwrap().to_lowercase() == "ethereum" {
            if let Some(ev) = def_file.get("event") {
                let ev = ev.as_str().unwrap();
                if ev != event {
                    let hashed_event = utils::get_ethereum_topic_ids(ev);
                    if hashed_event != event {
                        // println!("hashed_event: {}, event: {}", hashed_event, event);
                        continue;
                    }
                }
            } else {
                continue;
            }
        } else {
            // Non Ethereum Chains
            if
                def_file.get("event").unwrap().as_str().unwrap() != event ||
                def_file.get("chain_name").unwrap().as_str().unwrap().to_lowercase() !=
                    property.src_chain.clone().unwrap().to_lowercase()
            {
                // println!("Continuing...");
                continue;
            }
        }
        // Following files are all correct
        // Push onto checked Vec
        checked_vec.push(name.clone().to_owned());

        // Execute Custom Functions and get Variables
        match execute_custom_function(&def_file) {
            Ok(_) => {}
            Err(e) => {
                println!("Error: {}", e);
                return false;
            }
        }

        // parse pattern into AST
        let pattern = def_file.get("pattern").unwrap().as_array().unwrap();

        let patterns = pattern
            .iter()
            .map(|p| p.as_str().unwrap().to_string())
            .collect::<Vec<String>>();
        let mut line_results = vec![];
        for p in patterns {
            match build_ast_root(&p) {
                Ok(root) => {
                    root.print("");
                    match root.evaluate() {
                        Ok(v) => {
                            let ret: String = v.get_value();
                            line_results.push(ret);
                        }
                        Err(e) => {
                            println!("Error: {}", e);
                            line_results.push("false".to_string());
                            // return false;
                        }
                    }
                }
                Err(e) => {
                    println!("Error: {}", e);
                    line_results.push("false".to_string());
                    // return false;
                }
            }
        }

        // Join all line results with && in one string
        let processed_pattern = line_results
            .iter()
            .map(|p| p.as_str().to_string())
            .collect::<Vec<String>>()
            .join(" && ");

        // let processed_pattern = pattern
        //     .iter()
        //     .map(|p| p.as_str().unwrap().to_string())
        //     .collect::<Vec<String>>()
        //     .join(" && ");
        // // println!("Pattern: {}", processed_pattern);

        match build_ast_root(&processed_pattern) {
            Ok(root) => {
                // root.print("");

                // Evaluate AST
                let val = root.evaluate();
                match val {
                    Ok(v) => {
                        let ret: String = v.get_value();
                        println!("Pattern: {}", ret.fg::<DarkCyan>());
                        // Save result
                        if ret == "true" {
                            println!(
                                "{} transaction: {} From: {}",
                                "Allow".green(),
                                property.transaction_hash.clone().unwrap(),
                                name.yellow()
                            );
                            results.push(true);
                        } else {
                            println!(
                                "{} transaction: {} From: {}",
                                "Deny".red(),
                                property.transaction_hash.clone().unwrap(),
                                name.yellow()
                            );
                            fail_reason.push(name.clone().to_string());
                            results.push(false);
                        }
                    }
                    Err(e) => {
                        println!(
                            "{} transaction: {} From: {}",
                            "Deny".red(),
                            property.transaction_hash.clone().unwrap(),
                            name.yellow()
                        );
                        println!("Error: {}", e);
                        fail_reason.push(name.clone().to_string());
                        results.push(false);
                    }
                }
            }
            Err(e) => {
                println!("Error: {}", e);
                return false;
            }
        }
    }
    return true;
}

fn process_talon_code(
    property: Properties,
    results: &mut Vec<bool>,
    fail_reason: &mut Vec<String>
) {
    let event = property.occured_event.unwrap();

    for file in fs::read_dir("rules").unwrap() {
        let mut execute = false;
        let path = file.unwrap().path();
        match TalonFile::read_from_file(path.as_path()) {
            Ok(def_file) => {
                // Found a valid file
                // Check if the event matches
                if event == def_file.event {
                    println!("Found: {}", path.to_str().unwrap());
                    // Execute the code
                    execute = true;
                }
                if !execute {
                    let hashed_event = utils::get_ethereum_topic_ids(def_file.event.as_str());
                    if hashed_event != event {
                        execute = true;
                    }
                    // Skip wrong events
                    continue;
                }

                if execute {
                    let rules = def_file.rules;
                    let roots = build_code(&rules).unwrap();
                    println!("Roots: {:?}", roots);
                    for (l, root) in roots.iter().enumerate() {
                        root.print("");
                        // Evaluate AST
                        let val = root.evaluate();
                        match val {
                            Ok(v) => {
                                let ret: String = v.get_value();
                                println!("Rule: {}", ret.fg::<DarkCyan>());
                                // Save result
                                if ret == "true" {
                                    println!(
                                        "{} transaction: {} From: {}",
                                        "Allow".green(),
                                        property.transaction_hash.clone().unwrap(),
                                        def_file.name.yellow()
                                    );
                                    results.push(true);
                                } else {
                                    println!(
                                        "{} transaction: {} From: {} Line {}",
                                        "Deny".red(),
                                        property.transaction_hash.clone().unwrap(),
                                        def_file.name.yellow(),
                                        l
                                    );
                                    fail_reason.push(format!("{}: Line {}", def_file.name, l));
                                    results.push(false);
                                }
                            }
                            Err(e) => {
                                println!(
                                    "{} transaction: {} From: {} Line {}",
                                    "Deny".red(),
                                    property.transaction_hash.clone().unwrap(),
                                    def_file.name.yellow(),
                                    l
                                );
                                println!("Error: {}", e);
                                fail_reason.push(format!("{}: Line {}", def_file.name, l));
                                results.push(false);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                println!("Failed to read: {}", path.to_str().unwrap());
                println!("Error: {}", e);
            }
        }
    }
}

// Setup a TCP thread acting as a broadcaster for events
fn setup_event_ws(
    addr: String,
    event_queue: Arc<BlockingQueue<Event>>
) -> Result<(JoinHandle<()>, JoinHandle<()>), String> {
    // Build TCP Endpoint
    let listener = TcpListener::bind(addr).expect("Failed to bind Address");
    let connections: Arc<Mutex<Vec<TcpStream>>> = Arc::new(Mutex::new(Vec::new()));
    let connections_clone = Arc::clone(&connections);

    // Get Connections
    let connection_handler = thread::spawn(move || {
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

            for (id, mut x) in connections_clone2.lock().unwrap().iter_mut().enumerate() {
                match x.write_all(&serde_json::to_vec(&event).unwrap()) {
                    Ok(_) => {}
                    Err(e) =>
                        match e.kind() {
                            ErrorKind::ConnectionAborted | ErrorKind::ConnectionReset => {
                                x.shutdown(std::net::Shutdown::Both).unwrap();
                                connections_clone2.lock().unwrap().remove(id);
                            }
                            _ => println!("Error {}: {}", x.peer_addr().unwrap(), e),
                        }
                }
            }
            // connections_clone2.lock().unwrap().iter().for_each(|mut x| {
            //     match x.write_all(&serde_json::to_vec(&event).unwrap()) {
            //         Ok(_) => {},
            //         Err(e) => {
            //             match e.kind() {
            //             ErrorKind::ConnectionAborted | ErrorKind::ConnectionReset => {
            //                 x.shutdown(std::net::Shutdown::Both).unwrap();
            //                 connections_clone2.lock().unwrap().remove(x);
            //             }
            //             _ => println!("Error {}: {}", x.peer_addr().unwrap(), e),
            //             }
            //         }
            //     }
            // })
        }
    });
    // broadcast events

    Ok((connection_handler, event_handler))
}

/// Logging for evaluation
fn log_evaluation(evaluation: utils::Evaluation) {
    if !LOG_TIMESTAMPS.load(atomic::Ordering::Relaxed) {
        return;
    }
    evaluation.store();
}

#[test]
fn test_event_broadcast() {
    let event_queue: Arc<BlockingQueue<Event>> = Arc::new(BlockingQueue::new());
    let (handle1, handle2) = setup_event_ws(
        "127.0.0.1:8080".to_string(),
        event_queue.clone()
    ).unwrap();

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
fn test_receiving() {
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
