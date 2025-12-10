use std::sync::mpsc::Sender;

use owo_colors::OwoColorize;
use serde::{ Deserialize, Deserializer, Serialize };
use serde_json::{ json, Value };
use ws::Result;

use crate::{
    configs::connection::{ ConnectionConfig, get_established_connections },
    message_formats::solana_message::Res,
    properties::Properties,
    sockets::{ self, ethereum_socket, socket, solana_socket },
};

pub mod connection;
mod ethereum_config;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeConfig {
    pub contracts: Vec<ChainConfig>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainConfig {
    #[serde(rename = "subscription_method")]
    pub subscription_method: String,
    pub name: String,
    #[serde(rename = "contract_name")]
    pub contract_name: Option<String>,
    #[serde(rename = "contract_address")]
    pub contract_address: String,
    pub filter: Value,
}

// /// Configuration for connecting to a Blockchain and getting the events
// #[derive(Debug, Default, Deserialize, Serialize)]
// pub struct ChainConfig {
//     /// Name of the Blockchain
//     name: String,
//     /// RPC URL for Websocket
//     rpc_url: String,
//     /// HTTP URL for RPCs
//     http_url: String,
//     /// Contract Address on the Blockchain
//     contract_address: String,
//     /// Subscription Method Example: Ethereum "eth_subscribe" or Solana "logsSubscribe"
//     subscription_method: String,
//     /// Subscription Filter
//     filter: Value,
// }

impl ChainConfig {
    pub fn new(
        name: String,
        contract_name: Option<String>,
        contract_address: String,
        subscription_method: String,
        filter: Value
    ) -> Self {
        Self {
            name,
            contract_name,
            contract_address,
            subscription_method,
            filter,
        }
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

    // TODO: Check for replay if yes then connect to replay handlers

    fn connect_generic(&self, event_channel: Sender<Properties>) -> Result<()> {
        let request =
            json!({
            "jsonrpc": "2.0",
            "method": self.get_subscription_method(),
            "params": self.filter,
            "id": 1
        });

        // Check if Chain exists already
        if let Some(con) = connection::get_established_connections().get(&self.name) {
            println!("Chain {} is already connected", self.name);

            con.send(request.to_string()).unwrap();
            return Ok(());
        } else {
            //load config
            let connection_config: ConnectionConfig =
                ConnectionConfig::from_file("config/connections.json");
            let target_chain = connection_config.connections.iter().find(|x| x.name == self.name);
            if let Some(chain) = target_chain {
                ws::connect(chain.rpc_url.clone(), |out| {
                    let sender = connection
                        ::get_established_connections()
                        .insert(self.name.clone(), out);
                    match sender {
                        Some(o) => o.send(request.to_string()).unwrap(),
                        None => println!("No connection found for {}", self.name),
                    }
                    // Choose correct websocket implementation
                    // Process incoming WebSocket messages handled by the WebSocketClientHandler
                    socket::WebSocketClientHandler::new(
                        // State of the Client
                        self.name.clone(),
                        vec![]
                    )
                }).unwrap();
            } else {
                println!("No connection found for {}", self.name);
            }
        }

        Ok(())
    }

    fn connect_solana(&self, event_channel: Sender<Properties>) -> Result<()> {
        let request =
            json!({
            "jsonrpc": "2.0",
            "method": self.get_subscription_method(),
            "params": self.filter,
            "id": 1
        });
        // Check if Chain exists already
        if let Some(con) = connection::get_established_connections().get(&self.name) {
            println!("Chain {} is already connected", self.name);

            con.send(request.to_string()).unwrap();
            return Ok(());
        } else {
            //load config
            let connection_config: ConnectionConfig =
                ConnectionConfig::from_file("config/connections.json");
            let target_chain = connection_config.connections.iter().find(|x| x.name == self.name);
            if let Some(chain) = target_chain {
                ws::connect(chain.rpc_url.clone(), |out| {
                    let sender = connection
                        ::get_established_connections()
                        .insert(self.name.clone(), out);
                    match sender {
                        Some(o) => o.send(request.to_string()).unwrap(),
                        None => println!("No connection found for {}", self.name),
                    }

                    solana_socket::SolanaSocketHandler::new(
                        vec![],
                        event_channel.to_owned(),
                        chain.rpc_url.clone()
                    )
                }).unwrap();
            } else {
                println!("No connection found for {}", self.name);
            }
        }
        Ok(())
    }

    fn connect_ethereum(&self, event_channel: Sender<Properties>) -> Result<()> {
        let request =
            json!({
            "jsonrpc": "2.0",
            "method": self.get_subscription_method(),
            "params": self.filter,
            "id": 1
        });

        // // Check if Chain exists already
        // if let Some(con) = connection::get_established_connections().get(&self.name) {
        //     println!("Chain {} is already connected", self.name);
        //     println!("Request: {}", request);
        //     match con.send(request.to_string()){
        //         Ok(_) => println!("Request sent"),
        //         Err(e) => eprintln!("Error: {}", e),
        //     }
        //     return Ok(());
        // } else {
            println!("Making new connection to {}", self.name);
            //load config
            let connection_config: ConnectionConfig =
                ConnectionConfig::from_file("config/connections.json");
            let target_chain = connection_config.connections.iter().find(|x| x.name == self.name);
            if let Some(chain) = target_chain {
                ws::connect(chain.rpc_url.clone(), |out| {
                    let sender = connection
                        ::get_established_connections()
                        .insert(self.name.clone(), out);
                    match sender {
                        Some(o) => {
                            match o.send(request.to_string()) {
                                Ok(_) => println!("Request sent"),
                                Err(e) => eprintln!("Error: {}", e),
                            }
                        }
                        None => println!("No connection found for {}", self.name),
                    }
                    // Choose correct websocket implementation
                    // Process incoming WebSocket messages handled by the WebSocketClientHandler
                    ethereum_socket::EthereumSocketHandler::new(
                        vec![],
                        event_channel.to_owned(),
                        chain.rpc_url.clone()
                    )
                }).unwrap();
            } else {
                println!("No connection found for {}", self.name);
            }
        // }
        Ok(())
    }
}
