use std::{fs::{File, OpenOptions}, mem::MaybeUninit, sync::Once, path::Path, time::{Instant, Duration}, io::Write};

use chrono::{DateTime, Local, Datelike, Timelike};
use ethnum::{u256, uint, i256, int};

use crate::properties::Properties;

/// Convert hex string to u64 and remove leading 0x
pub fn hex_string_to_u64(hex_string: &str) -> u64 {
    let mut string_hex = hex_string;
    if string_hex.starts_with("0x"){
        string_hex = &string_hex[2..];
    }
    u64::from_str_radix(string_hex, 16).unwrap()
}

/// Convert hex string to u64 and remove leading 0x
pub fn hex_string_to_u128(hex_string: &str) -> u128 {
    let mut string_hex = hex_string;
    if string_hex.starts_with("0x"){
        string_hex = &string_hex[2..];
    }
    u128::from_str_radix(string_hex, 16).unwrap()
}

/// Convert a hex string to u256
pub fn hex_string_to_u256(hex_string: &str) -> u256 {
    let mut string_hex = hex_string;
    if string_hex.starts_with("0x"){
        string_hex = &string_hex[2..];
    }
    u256::from_str_radix(string_hex, 16).expect("Invalid hex string")
}

pub fn u256_to_hex_string(u256: u256) -> String {
    format!("0x{:X}", u256)
}

#[test]
fn test_hex_string_to_u256(){
    let hex = hex_string_to_u256("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
    println!("{}", hex);
    assert_eq!(hex, uint!("1393796574908163946345982392040522594123775"));
}

/// Convert a hex string to i256
pub fn hex_string_to_i256(hex_string: &str) -> i256 {
    let mut string_hex = hex_string;
    if string_hex.starts_with("0x"){
        string_hex = &string_hex[2..];
    }
    i256::from_str_radix(string_hex, 16).expect("Invalid hex string")
}

#[test]
fn test_hex_string_to_i256(){
    let hex = hex_string_to_i256("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");
    println!("{}", hex);
    assert_eq!(hex, int!("1393796574908163946345982392040522594123775"));
}

/// Convert u64 into hex string 0xXXXXX
pub fn u64_to_hex_string(u64: u64) -> String {
    format!("0x{:X}", u64)
}

/// Return the property and the index which match the hash
pub fn find_property_by_hash(hash: String, properties: &Vec<Properties>) -> Option<(Properties, u64)> {
    let mut id = 0;
    for property in properties.iter(){
        if property.transaction_hash == Some(hash.clone()){
            return Some((property.clone(), id));
        }
        id += 1;
    }
    None
}

/// Get the topic ids from a String
/// The string consists of the Events name followed by all parameters' types
/// E.g. "SendEthToSol(address,string,uint256)"
pub fn get_ethereum_topic_ids(event_header: &str) -> String {
    use sha3::Digest;
    
    let topic_id = sha3::Keccak256::digest(event_header.as_bytes()).to_vec();

    let hex_string: String = topic_id.iter().map(|&num| format!("{:02x}",num)).collect::<Vec<String>>().join("");

    let s = "0x".to_string() + &hex_string;
    //println!("{}", s);
    s
}

/// Get Startup instant
pub fn get_startup_time() -> &'static mut Instant {
    static mut MAYBE: MaybeUninit<Instant> = MaybeUninit::uninit();
    static ONLY: std::sync::Once = Once::new();

    unsafe{
        ONLY.call_once(|| {
            MAYBE.write(Instant::now());
        });
        MAYBE.assume_init_mut()
    }
}

/// Get the handle of the log file
pub fn get_log_file() -> &'static mut File {
    static mut MAYBE: MaybeUninit<File> = MaybeUninit::uninit();
    static ONLY: std::sync::Once = Once::new();

    unsafe{
        ONLY.call_once(|| {
            let mut f: File;
            let current_datetime: DateTime<Local> = Local::now();
            let year = current_datetime.year();
            let month = current_datetime.month();
            let day = current_datetime.day();
            let hour = current_datetime.hour();
            let minute = current_datetime.minute();
            let second = current_datetime.second();
            let date_time = format!("{}-{:02}-{:02}_{:02}_{:02}_{:02}", year, month, day, hour, minute, second);
            let path_rel = format!("logs/{}.log", date_time);

            let temp = File::create(path_rel.clone());
            match temp {
                Ok(file) => f = file,
                Err(e) => {
                    panic!("Could not create log file {}: {}", path_rel, e);
                }
            }
            let header = "ID;Event;Duration;Pattern Duration\n";
            match f.write_all(header.as_bytes()){
                Ok(_) => {},
                Err(e) => {
                    panic!("Could not write header to log file: {}", e);
                }
            }
            
            // match Path::new(&path_rel).canonicalize() {
            //     Ok(path) => {
            //         match OpenOptions::new().create(true).append(true).open(path.clone()) {
            //             Ok(file) => f = file,
            //             Err(e) => {
            //                 panic!("Could not open log file{}: {}", path.to_str().unwrap(), e);
            //             }
            //         }   
            //     },
            //     Err(e) => {
            //         panic!("Could not open log file {}: {}", path_rel, e);
            //     }
            // }
            MAYBE.write(f);
        });
        MAYBE.assume_init_mut()
    }
}

#[test]
fn test_topic_ids_ethereum(){
    let event_header = "InconsistentDepositLogic(uint256,string,uint256)";
    let topic_id = get_ethereum_topic_ids(event_header);
    println!("{}", topic_id);
}

// TODO: Implement more logs for the evaluation
#[derive(Debug, Clone, Default)]
pub struct Evaluation {
    pub id: u64,
    pub event_type: String,
    pub duration: u128,
    pub pattern_duration: u128,
    pub pattern_type: String,
    pub contract_address: String,
    pub msg_sender: String,
    pub block_number: String,
    pub msg_value: u256,
}

impl Evaluation {
    pub fn store(&self) -> bool {
        let csv_string = format!("{};{};{};{}\n", self.id, self.event_type, self.duration, self.pattern_duration);
        let mut f = get_log_file();
        match f.write_all(csv_string.as_bytes()) {
            Ok(_) => {return true},
            Err(e) => {
                eprintln!("Error when writing to file: {}", e);
                return false
            },
        }
    }
}