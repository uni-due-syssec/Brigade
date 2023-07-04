use serde_derive::Deserialize;
use serde_derive::Serialize;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolanaSubscription {
    pub jsonrpc: String,
    pub id: i64,
    pub method: String,
    pub params: Vec<SolanaSubscriptionParams>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolanaSubscriptionParams {
    #[serde(default)]
    pub mentions: Vec<String>,
    pub commitment: Option<String>,
}
