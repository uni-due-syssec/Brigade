use ws::Handler;
use serde_json::Value;
use crate::{message_formats::ethereum_message::{EthereumEventMessage, EthereumConfirmMessage}};

/// The Endpoint Client for the Blockchain Smart Contracts
/// Here a Handler will fetch and process the events and take care of the websocket connection
pub struct WebSocketClientHandler{
    // State of the Client
    pub(crate) sender: ws::Sender,
    pub(crate) chain_name: String,
}

impl WebSocketClientHandler {
    pub fn new(chain_name: String, sender: ws::Sender) -> Self {
        Self {
            chain_name,
            sender,
        }
    }

    fn handle_ethereum(&self, message: Value) {
        if let Ok(ethereum_msg) = serde_json::from_value::<EthereumEventMessage>(message.clone()) {
            println!("Ethereum Message: {}", ethereum_msg);
            println!("Current Block: {}", ethereum_msg.params.result.block_number);

            let mut string_block_number = ethereum_msg.params.result.block_number.as_str();
            if string_block_number.starts_with("0x"){
                string_block_number = &string_block_number[2..];
            }
            let current_block = u64::from_str_radix(string_block_number, 16).unwrap();

            // Get Transaction by Hash
            println!("Getting Transaction by Hash: {} at Block {}", "Huge", current_block);
            let get_transaction_by_hash = format!(
                r#"{{
                    "jsonrpc": "2.0",
                    "method": "eth_getTransactionByHash",
                    "params": ["{}"],
                    "id": 1
                }}"#,
                ethereum_msg.params.result.transaction_hash
            );
            
            self.sender.send(get_transaction_by_hash.to_string()).unwrap();

            println!("Getting Balance of Address: {} at Block {}", "Huge", current_block);
            // TODO: get the balance of the Account 
            let get_balance_at_block = format!(
                r#"{{
                    "jsonrpc": "2.0",
                    "method": "eth_getBalance",
                    "params": ["{}","{}"],
                    "id": 1
                }}"#,
                ethereum_msg.params.result.address,
                current_block
            );

            println!("Getting Balance of Address: {} at previous Block {}", "Huge", current_block - 1);
            // TODO: get the balance of the Account


        }else if let Ok(ethereum_confirm_msg) = serde_json::from_value::<EthereumConfirmMessage>(message.clone()) {
            println!("Ethereum Confirm Message: {}", ethereum_confirm_msg);
        }else{
            if let Ok(pretty_json) = serde_json::to_string_pretty(&message) {
                // Print the pretty-printed JSON string
                println!("{}", pretty_json);
            }else{
                println!("Invalid JSON");
            }
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
                self.handle_ethereum(message);
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