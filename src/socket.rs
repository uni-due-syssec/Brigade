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

    fn handle_ethereum(&mut self, message: Value) {
        if let Ok(ethereum_msg) = serde_json::from_value::<EthereumEventMessage>(message.clone()) {

            // A new Event is emitted --> A new Index in the properties list must be added
            self.properties.push(Properties::new());
            let index = self.properties.len() - 1;
            self.properties[index].transaction_hash = Some(ethereum_msg.params.result.transaction_hash.clone());
            self.properties[index].block_number = Some(ethereum_msg.params.result.block_number.clone());
            self.properties[index].occured_event = Some(ethereum_msg.params.result.topics[0].clone());
            
            println!("Ethereum Message: {}", ethereum_msg);
            
            // Get Transaction by Hash
            let get_transaction_by_hash = format!(
                r#"{{
                    "jsonrpc": "2.0",
                    "method": "eth_getTransactionByHash",
                    "params": ["{}"],
                    "id": {}
                }}"#,
                ethereum_msg.params.result.transaction_hash,
                index
            );
            
            // Send the message to the websocket
            self.sender.send(get_transaction_by_hash.to_string()).unwrap();


        }else if let Ok(ethereum_confirm_msg) = serde_json::from_value::<EthereumConfirmMessage>(message.clone()) {
            println!("Ethereum Confirm Message: {}", ethereum_confirm_msg);
        }else if let Ok(ethereum_transaction_by_hash) = serde_json::from_value::<EthereumTransactionByHash>(message.clone()) {
            println!("Ethereum Transaction by Hash: {}", ethereum_transaction_by_hash);
            let index = ethereum_transaction_by_hash.id;
            // Find the Property File by id of transaction
            let property = &mut self.properties[index as usize];
            property.value = Some(ethereum_transaction_by_hash.result.value.clone());
            property.payer_address = Some(ethereum_transaction_by_hash.result.from.clone());
            
            // Get the payers Balance before and after the block
            println!("Current Block: {}", ethereum_transaction_by_hash.result.block_number);

            let current_block = utils::hex_string_to_u64(ethereum_transaction_by_hash.result.block_number.as_str());

            println!("Getting Balance of Address: {} at Block {}", ethereum_transaction_by_hash.result.from, current_block);
            // Build JsonRPC Request
            let get_balance_at_block = format!(
                r#"{{
                    "jsonrpc": "2.0",
                    "method": "eth_getBalance",
                    "params": ["{}","{}"],
                    "id": {} 
                }}"#,
                ethereum_transaction_by_hash.result.from.clone(),
                utils::u64_to_hex_string(current_block),
                index
            );

            // Send the message to the websocket
            self.sender.send(get_balance_at_block.to_string()).unwrap();

            println!("Getting Balance of Address: {} at previous Block {}", ethereum_transaction_by_hash.result.from, current_block - 1);
            // Build JsonRPC Request
            let get_balance_before_block = format!(
                r#"{{
                    "jsonrpc": "2.0",
                    "method": "eth_getBalance",
                    "params": ["{}","{}"],
                    "id": {}
                }}"#,
                ethereum_transaction_by_hash.result.from.clone(),
                utils::u64_to_hex_string(current_block-1),
                index
            );
            // Send the message to the websocket
            self.sender.send(get_balance_before_block.to_string()).unwrap();

        }else if let Ok(ethereum_balance_msg) = serde_json::from_value::<EthereumBalanceMessage>(message.clone()) {
            println!("Ethereum Balance Message: {}", ethereum_balance_msg);
            println!("Balance of Account: {}", ethereum_balance_msg.result);

            let index = ethereum_balance_msg.id;
            if index > self.properties.len().try_into().unwrap() { // Startup. Ok Messages for Subscriptions
                
            }else { // Process the balance
                let property = &mut self.properties[index as usize];
                if property.payer_balance_after.is_none() {
                    property.payer_balance_after = Some(ethereum_balance_msg.result);
                }else {
                    if property.payer_balance_before.is_none() {
                        property.payer_balance_before = Some(ethereum_balance_msg.result);
                    }
                }

                println!("Property: {:?}", property);

                // Check for all properties in the property descpription folder
                property.check().unwrap();
            }

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