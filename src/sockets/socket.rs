use ws::Handler;
use serde_json::Value;
use crate::{message_formats::ethereum_message::{EthereumEventMessage, EthereumConfirmMessage, EthereumTransactionByHash, EthereumBalanceMessage}, properties::Properties};
use crate::utils;

/// The Endpoint Client for the Blockchain Smart Contracts
/// Here a Handler will fetch and process the events and take care of the websocket connection
pub struct WebSocketClientHandler{
    // State of the Client
    pub(crate) sender: ws::Sender,
    pub(crate) chain_name: String,
    pub(crate) properties: Vec<Properties>,
}

impl WebSocketClientHandler {
    pub fn new(chain_name: String, sender: ws::Sender, properties: Vec<Properties>) -> Self {
        Self {
            chain_name,
            sender,
            properties
        }
    }
}

/// Here the WebSocket Handles the basic workflow
impl Handler for WebSocketClientHandler {
    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {

        // Try to parse the message into ethereum message
        let message: Value = serde_json::from_str(&msg.to_string()).unwrap();

        match self.chain_name.as_str(){
            "solana" | "Solana" => {
                println!("Solana Message: {}", message);
            },
            "ethereum" | "Ethereum" => {
                println!("Ethereum Message: {}", message);
                println!("For better parsing check the EthereumWebsocketHandler");
            }
            _ => {
                println!("Custom Message: {}", message);
            },
        }

        Ok(())
    }

    fn on_open(&mut self, _shake: ws::Handshake) -> ws::Result<()> {
        println!("Open");
        Ok(())
    }
}