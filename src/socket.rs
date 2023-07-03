use ws::Handler;
use serde_json::Value;
use crate::{message_formats::ethereum_message::{EthereumMessage, EthereumConfirmMessage}};

/// The Endpoint Client for the Blockchain Smart Contracts
/// Here a Handler will fetch and process the events and take care of the websocket connection
pub struct WebSocketClientHandler{
    // State of the Client
    pub(crate) chain_name: String,
}

impl WebSocketClientHandler {
    pub fn new(chain_name: String) -> Self {
        Self {
            chain_name,
        }
    }
}

impl Handler for WebSocketClientHandler {
    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {

        // Try to parse the message into ethereum message
        let message: Value = serde_json::from_str(&msg.to_string()).unwrap();

        match self.chain_name.as_str(){
            "solana" | "Solana" => {
                println!("Solana Message: {}", message);
            },
            "ethereum" | "Ethereum" => {
                handle_ethereum(message);
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

fn handle_ethereum(message: Value) {
    if let Ok(ethereum_msg) = serde_json::from_value::<EthereumMessage>(message.clone()) {
        println!("Ethereum Message: {}", ethereum_msg);
    }else if let Ok(ethereum_confirm_msg) = serde_json::from_value::<EthereumConfirmMessage>(message.clone()) {
        println!("Ethereum Confirm Message: {}", ethereum_confirm_msg);
    }
    else{
        println!("None Ethereum Message");
        if let Ok(pretty_json) = serde_json::to_string_pretty(&message) {
            // Print the pretty-printed JSON string
            println!("{}", pretty_json);
        }else{
            println!("Invalid JSON");
        }
    }
}