
use std::sync::mpsc::Sender;

use serde::{Deserialize, Serialize, Deserializer};
use serde_json::{json, Value};
use ws::Result;

use crate::{sockets::{self, socket, ethereum_socket, solana_socket}, properties::Properties};

mod ethereum_config;

/// Configuration for connecting to a Blockchain and getting the events
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ChainConfig {
    /// Name of the Blockchain
    name: String,
    /// RPC URL for Websocket
    rpc_url: String,
    /// HTTP URL for RPCs
    http_url: String,
    /// Contract Address on the Blockchain
    contract_address: String,
    /// Subscription Method Example: Ethereum "eth_subscribe" or Solana "logsSubscribe"
    subscription_method: String,
    /// Subscription Filter
    filter: Value,
}

impl ChainConfig {
    pub fn new(rpc_url: String, http_url: String, name: String, contract_address: String, subscription_method: String, filter: Value) -> Self {
        Self {
            rpc_url,
            http_url,
            name,
            contract_address,
            subscription_method,
            filter,
        }
    }
    
    pub fn get_http_url(&self) -> String {
        self.http_url.clone()
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

    pub fn connect(&self, event_channel: Sender<Properties>) -> Result<()> {
        match self.name.to_lowercase().as_str() {
            "solana" => self.connect_solana(event_channel),
            "ethereum" => self.connect_ethereum(event_channel),
            _ => self.connect_generic(event_channel),
        }
    }

    fn connect_generic(&self, event_channel: Sender<Properties>) -> Result<()> {
        ws::connect(self.rpc_url.clone(), |out| {
            // Request subscription for Chain Events
            let request = json!({
                "jsonrpc": "2.0",
                "method": self.get_subscription_method(),
                "params": self.filter,
                "id": 1
            });
            
            // println!("{:?}", request.to_string());

            out.send(request.to_string()).unwrap();
            // Choose correct websocket implementation
            // Process incoming WebSocket messages handled by the WebSocketClientHandler
            socket::WebSocketClientHandler::new(
                // State of the Client
                self.name.clone(),out, vec![])
        }).unwrap();
        
        Ok(())
    }

    fn connect_solana(&self, event_channel: Sender<Properties>) -> Result<()> {
        ws::connect(self.rpc_url.clone(), |out| {
            // Request subscription for Chain Events
            let request = json!({
                "jsonrpc": "2.0",
                "method": self.get_subscription_method(),
                "params": self.filter,
                "id": 1
            });

            out.send(request.to_string()).unwrap();

            solana_socket::SolanaSocketHandler::new(out, vec![], event_channel.to_owned(), self.get_http_url().clone())
        }).unwrap();
        
        Ok(())
    }

    fn connect_ethereum(&self, event_channel: Sender<Properties>) -> Result<()> {
        ws::connect(self.rpc_url.clone(), |out| {
            // Request subscription for Chain Events
            let request = json!({
                "jsonrpc": "2.0",
                "method": self.get_subscription_method(),
                "params": self.filter,
                "id": 1
            });
            
            // println!("{:?}", request.to_string());

            out.send(request.to_string()).unwrap();
            // Choose correct websocket implementation
            // Process incoming WebSocket messages handled by the WebSocketClientHandler
            ethereum_socket::EthereumSocketHandler::new(out, vec![], event_channel.to_owned(), self.get_http_url().clone())
        }).unwrap();
        
        Ok(())
    }
}