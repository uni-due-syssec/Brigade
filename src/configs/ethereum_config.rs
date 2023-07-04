use serde::Deserialize;
use serde::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthereumSubscription {
    pub jsonrpc: String,
    pub id: i64,
    pub method: String,
    pub params: EthereumSubscriptionParams,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EthereumSubscriptionParams {
    pub address: String,
    pub topics: Vec<String>,
}
