use std::{sync::{Mutex, Arc, Once}, mem::MaybeUninit, thread::JoinHandle};

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
    u128::from_str_radix(hex_string, 16).unwrap()
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

    let event_header = "SendEthToSol(address,string,uint256)";
    let topic_id = sha3::Keccak256::digest(event_header.as_bytes()).to_vec();

    let hex_string: String = topic_id.iter().map(|&num| format!("{:02x}",num)).collect::<Vec<String>>().join("");

    let s = "0x".to_string() + &hex_string;
    //println!("{}", s);
    s
}