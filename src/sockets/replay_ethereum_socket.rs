use std::collections::HashSet;
use std::sync::mpsc::Sender;

use ethnum::u256;
use reqwest::blocking::{get, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::get_variable_map_instance;
use crate::properties::ast::build_ast_root;
use crate::utils::get_startup_time;
use crate::VarValues;
use crate::{message_formats::ethereum_message::*, properties::Properties, set_var, utils};

pub struct ReplayEthereumSocketHandler {
    // State of the Client
    pub(crate) chain_name: String,
    pub(crate) topics: HashSet<String>,
}

impl ReplayEthereumSocketHandler {
    // The function retrieves all transactions with a specified log
    // The log is the keccak256(event) where event is the abi encoded version of a Solidity Event.
    pub fn get_all_transactions_with_log(&self, log: Vec<String>) -> Vec<Properties> {
        let mut logs: Vec<&str> = log.iter().map(|s| s.as_str()).collect();
        let l = format!("{:?}", logs);
        let call = format!("call(ethereum, get_logs, [{:?}])", vec![l]); // Blocknumber
        println!("Call: {}", call);
        let root = build_ast_root(call.as_str()).unwrap();
        root.print("");
        let val = root.evaluate().unwrap();
        print!("Val: {:?}\n", val);
        let log_instance = Value::from(VarValues::from(val));
        let mut hashes = Vec::new();
        find_transaction_hashes(&log_instance, &mut hashes);
        self.find_corresponding_transaction(hashes)
    }

    pub fn retrieve_block(&self, block_number: u256) -> Value {
        let call = format!(
            "call(ethereum, get_block_by_number, [{}]).get(result)",
            format!("{:#x}", block_number)
        ); // Blocknumber
        println!("Call: {}", call);
        let root = build_ast_root(call.as_str()).unwrap();
        root.print("");
        let val = root.evaluate().unwrap();
        print!("Val: {:?}\n", val);
        Value::from(VarValues::from(val))
    }

    fn find_corresponding_transaction(&self, hashes: Vec<String>) -> Vec<Properties> {
        let mut properties: Vec<Properties> = Vec::new();
        for h in hashes {
            let call = format!(
                "call(ethereum, get_transaction_receipt, [{}]).get(result)",
                h
            ); // Blocknumber

            let root = build_ast_root(call.as_str()).unwrap();
            root.print("");
            let val = root.evaluate().unwrap();
            let receipt = Value::from(VarValues::from(val));

            // check if topics match
            if let Some(logs) = receipt.get("logs").and_then(|l| l.as_array()) {
                for log in logs {
                    if let Some(removed) = log.get("removed") {
                        // If the log was removed due to consensus we skip it
                        if removed.as_bool().unwrap() {
                            continue;
                        }
                    }
                    if let Some(_topics) = log.get("topics").and_then(|t| t.as_array()) {
                        if _topics
                            .iter()
                            .any(|t| self.topics.contains(t.as_str().unwrap()))
                        {
                            let topnum = _topics
                                .iter()
                                .find(|t| self.topics.contains(t.as_str().unwrap()))
                                .and_then(|t| Some(t.as_str().unwrap().to_string()));
                            let payer = receipt.get("from").unwrap().as_str().unwrap();

                            let tx = get_transaction_by_hash(h.clone());
                            let value = tx.get("value").unwrap().as_str().unwrap();
                            let block = u256::from_str_hex(
                                log.get("blockNumber").unwrap().as_str().unwrap(),
                            )
                            .unwrap();
                            let payer_balance_before =
                                get_balance_at_block(payer.to_string(), block - 1);
                            let payer_balance_after =
                                get_balance_at_block(payer.to_string(), block);

                            let p = Properties {
                                occured_event: topnum,
                                transaction_hash: Some(h.clone()),
                                block_number: Some(block),
                                payer_address: Some(payer.to_string()),
                                payer_balance_before: Some(payer_balance_before),
                                payer_balance_after: Some(payer_balance_after),
                                value: Some(u256::from_str_hex(value).unwrap()),
                                src_chain: Some("ethereum".to_string()),
                            };
                            properties.push(p);
                        }
                    }
                }
            }
        }
        properties
    }

    pub fn filter_block(&self, block: Value) -> Vec<Properties> {
        // for each transaction hash in the block, get the transaction
        let mut hashes = Vec::new();
        // find_transaction_hashes(&block, &mut hashes);
        let hash_val = block
            .get("transactions")
            .expect("Wrong format for block transactions")
            .as_array()
            .unwrap()
            .to_vec();
        for h in hash_val {
            hashes.push(h.as_str().unwrap().to_string());
        }
        self.find_corresponding_transaction(hashes)
    }
}

/// Recursively searches through a JSON `Value` for keys named "transactionHash" and
/// collects the corresponding values as strings into the provided `hashes` vector.
///
/// # Arguments
///
/// * `value` - A reference to a `serde_json::Value` which may contain transaction hashes.
/// * `hashes` - A mutable reference to a vector of strings where found transaction hashes
///   will be collected.
fn find_transaction_hashes(value: &Value, hashes: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                if k == "transactionHash" {
                    if let Some(s) = v.as_str() {
                        hashes.push(s.to_string());
                    }
                }
                find_transaction_hashes(v, hashes);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                find_transaction_hashes(v, hashes);
            }
        }
        _ => {}
    }
}

/// Given a transaction hash, returns the transaction receipt from the Ethereum node.
///
/// # Arguments
///
/// * `hash` - The transaction hash to look up.
///
/// # Returns
///
/// A `Value` containing the transaction receipt.
fn get_transaction_by_hash(hash: String) -> Value {
    let call = format!(
        "call(ethereum, get_transaction_by_hash, [{}]).get(result)",
        hash
    ); // Blocknumber

    let root = build_ast_root(call.as_str()).unwrap();
    root.print("");
    let val = root.evaluate().unwrap();
    let tx = Value::from(VarValues::from(val));
    tx
}

/// Given an Ethereum address and block number, returns the balance of that address at
/// the specified block number.
///
/// # Arguments
///
/// * `address` - The Ethereum address to look up.
/// * `block_number` - The block number at which to look up the balance.
///
/// # Returns
///
/// A `u256` containing the balance of the address at the specified block number.
fn get_balance_at_block(address: String, block_number: u256) -> u256 {
    let call = format!(
        "call(ethereum, get_balance, [\"{}\", {}]).get(result)",
        address, block_number
    ); // Blocknumber

    let root = build_ast_root(call.as_str()).unwrap();
    root.print("");
    let val = root.evaluate().unwrap();
    let balance = Value::from(VarValues::from(val));
    u256::from_str_hex(balance.as_str().unwrap()).unwrap() // Return the balance
}

/// The serialized version of a Config for the replay
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ReplayConfig {
    /// Topics to be subscribed to
    pub topics: Vec<String>,
}

#[test]
fn test_replay_config_deser() {
    let json = r#"{"topics":["0x123456","0x7890ab"]}"#;
    let config: ReplayConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.topics, vec!["0x123456", "0x7890ab"]);

    let json2 = serde_json::to_string(&config).unwrap();
    assert_eq!(json, json2);

    println!("{:?}", json);
}
