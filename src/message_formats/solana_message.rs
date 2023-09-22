use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogMessage {
    pub jsonrpc: String,
    pub method: String,
    pub params: Params,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Params {
    pub result: Res,
    pub subscription: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Res {
    pub context: Ctx,
    pub value: Val,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ctx {
    pub slot: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Val {
    pub signature: String,
    pub err: Value,
    pub logs: Vec<String>,
}
// **************************************
// Solana Transaction Message
// **************************************
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionMessage {
    pub jsonrpc: String,
    pub result: TransactionResult,
    pub id: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionResult {
    pub block_time: i64,
    pub meta: Meta,
    pub slot: i64,
    pub transaction: TransactionTx,
    pub version: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub compute_units_consumed: i64,
    pub err: Value,
    pub fee: i64,
    pub inner_instructions: Vec<Value>,
    pub log_messages: Vec<String>,
    pub post_balances: Vec<i64>,
    pub post_token_balances: Vec<Value>,
    pub pre_balances: Vec<i64>,
    pub pre_token_balances: Vec<Value>,
    pub rewards: Vec<Value>,
    pub status: TransactionStatus,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionStatus {
    #[serde(rename = "Ok")]
    pub ok: Value,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionTx {
    pub message: SubMessage,
    pub signatures: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubMessage {
    pub account_keys: Vec<AccountKey>,
    pub address_table_lookups: Vec<Value>,
    pub instructions: Vec<Instruction>,
    pub recent_blockhash: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountKey {
    pub pubkey: String,
    pub signer: bool,
    pub source: String,
    pub writable: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instruction {
    pub accounts: Vec<String>,
    pub data: String,
    pub program_id: String,
}
