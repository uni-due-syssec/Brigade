
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use ws::Result;

use crate::socket;

/// Configuration for connecting to a Blockchain and getting the events
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ChainConfig {
    /// Name of the Blockchain
    name: String,
    /// Chain ID
    chain_id: u64,
    /// RPC URL for Websocket
    rpc_url: String,
    /// Contract Address on the Blockchain
    contract_address: String,
    /// Subscription Method Example: Ethereum "eth_subscribe" or Solana "logsSubscribe"
    subscription_method: String,
    /// Subscription Filter
    filter: Value,
}

impl ChainConfig {
    pub fn new(chain_id: u64, rpc_url: String, name: String, contract_address: String, subscription_method: String, filter: Value) -> Self {
        Self {
            chain_id,
            rpc_url,
            name,
            contract_address,
            subscription_method,
            filter,
        }
    }

    pub fn get_chain_id(&self) -> u64 {
        self.chain_id
    }

    pub fn get_rpc_url(&self) -> String {
        self.rpc_url.clone()
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn get_contract_address(&self) -> String {
        self.contract_address.clone()
    }

    fn get_subscription_method(&self) -> String {
        self.subscription_method.clone()
    }

    pub fn connect(&self) -> Result<()> {
        ws::connect(self.rpc_url.clone(), |out| {
            // // Request subscription for Chain Events

            let request = json!({
                "jsonrpc": "2.0",
                "method": self.get_subscription_method(),
                "params": self.filter,
                "id": 1
            });
            
            println!("{:?}", request.to_string());

            out.send(request.to_string()).unwrap();
    
            // Process incoming WebSocket messages handled by the WebSocketClientHandler
            socket::WebSocketClientHandler{
                // State of the Client
            }
        }).unwrap();

        Ok(())
    }
}