use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthereumEventMessage {
    pub jsonrpc: String,
    pub method: String,
    pub params: EventParams,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventParams {
    pub result: EventResult,
    pub subscription: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventResult {
    pub address: String,
    pub block_hash: String,
    pub block_number: String,
    pub data: String,
    pub log_index: String,
    pub removed: bool,
    pub topics: Vec<String>,
    pub transaction_hash: String,
    pub transaction_index: String,
}
impl fmt::Display for EthereumEventMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text = serde_json::to_string_pretty(self).unwrap();
        write!(f, "{}", text)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthereumConfirmMessage {
    pub id: i64,
    pub jsonrpc: String,
    pub result: i64,
}

impl fmt::Display for EthereumConfirmMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text = serde_json::to_string_pretty(self).unwrap();
        write!(f, "{}", text)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthereumTransactionByHash {
    pub id: i64,
    pub jsonrpc: String,
    pub result: EthereumTransactionByHashResult,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthereumTransactionByHashResult {
    pub access_list: Vec<Value>,
    pub block_hash: String,
    pub block_number: String,
    pub chain_id: String,
    pub from: String,
    pub gas: String,
    pub gas_price: String,
    pub hash: String,
    pub input: String,
    pub max_fee_per_gas: String,
    pub max_priority_fee_per_gas: String,
    pub nonce: String,
    pub r: String,
    pub s: String,
    pub to: String,
    pub transaction_index: String,
    #[serde(rename = "type")]
    pub type_field: String,
    pub v: String,
    pub value: String,
}

impl fmt::Display for EthereumTransactionByHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text = serde_json::to_string_pretty(self).unwrap();
        write!(f, "{}", text)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthereumBalanceMessage {
    pub id: i64,
    pub jsonrpc: String,
    pub result: String,
}

impl fmt::Display for EthereumBalanceMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text = serde_json::to_string_pretty(self).unwrap();
        write!(f, "{}", text)
    }
}