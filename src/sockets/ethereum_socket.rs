use std::sync::mpsc::Sender;

use serde_json::Value;
use ws::Handler;
use reqwest::blocking::Client;

use crate::{properties::Properties, utils, message_formats::ethereum_message::*};

/// Ethereum Websocket Handler
pub struct EthereumSocketHandler{
    // State of the Client
    pub(crate) sender: ws::Sender,
    pub(crate) chain_name: String,
    pub(crate) properties: Vec<Properties>,
    pub(crate) event_channel: Sender<String>,
    request_url: String,
}

impl EthereumSocketHandler {
    pub fn new(sender: ws::Sender, properties: Vec<Properties>, event_channel: Sender<String>, request_url: String) -> Self {
        Self {
            chain_name: "ethereum".to_string(),
            sender,
            properties,
            event_channel,
            request_url,
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
            
            // println!("Ethereum Message: {}", ethereum_msg);
            
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

            // Build HTTP Post for Transaction Data
            let client = Client::new();
            let mut request_body: Value = serde_json::from_str(get_transaction_by_hash.as_str()).unwrap();
            let res = client.post(self.request_url.clone()).json(&request_body).send().unwrap();

            let body = res.text().unwrap();
            let transaction_by_hash = serde_json::from_str::<EthereumTransactionByHash>(&body.as_str()).unwrap();

            self.properties[index].value = Some(transaction_by_hash.result.value.clone());
            self.properties[index].payer_address = Some(transaction_by_hash.result.from.clone());
            // Get Current Block as decimal u64
            let current_block = utils::hex_string_to_u64(transaction_by_hash.result.block_number.as_str());

            let get_balance_at_block = format!(
r#"{{
    "jsonrpc": "2.0",
    "method": "eth_getBalance",
    "params": ["{}","{}"],
    "id": {} 
}}"#,
                transaction_by_hash.result.from.clone(),
                utils::u64_to_hex_string(current_block),
                index
            );

            request_body = serde_json::from_str(get_balance_at_block.as_str()).unwrap();
            let res = client.post(self.request_url.clone()).json(&request_body).send().unwrap();
            let body = res.text().unwrap();
            let balance_at_block = serde_json::from_str::<EthereumBalanceMessage>(&body).unwrap();
            self.properties[index].payer_balance_after = Some(balance_at_block.result.clone());

            let get_balance_before_block = format!(
r#"{{
    "jsonrpc": "2.0",
    "method": "eth_getBalance",
    "params": ["{}","{}"],
    "id": {} 
}}"#,
                transaction_by_hash.result.from.clone(),
                utils::u64_to_hex_string(current_block-1),
                index
            );
            request_body = serde_json::from_str(get_balance_before_block.as_str()).unwrap();
            let res = client.post(self.request_url.clone()).json(&request_body).send().unwrap();
            let body = res.text().unwrap();
            let balance_before_block = serde_json::from_str::<EthereumBalanceMessage>(&body).unwrap();
            self.properties[index].payer_balance_before = Some(balance_before_block.result.clone());

            println!("Properties full: {:?}", self.properties[index]);
            self.event_channel.send(self.properties[index].occured_event.clone().unwrap()).unwrap();

            // // Send the message to the websocket
            // self.sender.send(get_transaction_by_hash.to_string()).unwrap();


        }else if let Ok(ethereum_confirm_msg) = serde_json::from_value::<EthereumConfirmMessage>(message.clone()) {
            println!("Ethereum Confirm Message: {}", ethereum_confirm_msg);
        // }else if let Ok(ethereum_transaction_by_hash) = serde_json::from_value::<EthereumTransactionByHash>(message.clone()) {
        //     println!("Ethereum Transaction by Hash: {}", ethereum_transaction_by_hash);
        //     let index = ethereum_transaction_by_hash.id;
        //     // Find the Property File by id of transaction
        //     let property = &mut self.properties[index as usize];
        //     property.value = Some(ethereum_transaction_by_hash.result.value.clone());
        //     property.payer_address = Some(ethereum_transaction_by_hash.result.from.clone());
            
        //     // Get the payers Balance before and after the block
        //     println!("Current Block: {}", ethereum_transaction_by_hash.result.block_number);

        //     let current_block = utils::hex_string_to_u64(ethereum_transaction_by_hash.result.block_number.as_str());

        //     println!("Getting Balance of Address: {} at Block {}", ethereum_transaction_by_hash.result.from, current_block);
        //     // Build JsonRPC Request
        //     let get_balance_at_block = format!(
        //         r#"{{
        //             "jsonrpc": "2.0",
        //             "method": "eth_getBalance",
        //             "params": ["{}","{}"],
        //             "id": {} 
        //         }}"#,
        //         ethereum_transaction_by_hash.result.from.clone(),
        //         utils::u64_to_hex_string(current_block),
        //         index
        //     );

        //     // Send the message to the websocket
        //     self.sender.send(get_balance_at_block.to_string()).unwrap();

        //     println!("Getting Balance of Address: {} at previous Block {}", ethereum_transaction_by_hash.result.from, current_block - 1);
        //     // Build JsonRPC Request
        //     let get_balance_before_block = format!(
        //         r#"{{
        //             "jsonrpc": "2.0",
        //             "method": "eth_getBalance",
        //             "params": ["{}","{}"],
        //             "id": {}
        //         }}"#,
        //         ethereum_transaction_by_hash.result.from.clone(),
        //         utils::u64_to_hex_string(current_block-1),
        //         index
        //     );
        //     // Send the message to the websocket
        //     self.sender.send(get_balance_before_block.to_string()).unwrap();

        // }else if let Ok(ethereum_balance_msg) = serde_json::from_value::<EthereumBalanceMessage>(message.clone()) {
        //     println!("Ethereum Balance Message: {}", ethereum_balance_msg);
        //     println!("Balance of Account: {}", ethereum_balance_msg.result);

        //     let index = ethereum_balance_msg.id;
        //     if index > self.properties.len().try_into().unwrap() { // Startup. Ok Messages for Subscriptions
                
        //     }else { // Process the balance
        //         let property = &mut self.properties[index as usize];
        //         if property.payer_balance_after.is_none() {
        //             property.payer_balance_after = Some(ethereum_balance_msg.result);
        //         }else {
        //             if property.payer_balance_before.is_none() {
        //                 property.payer_balance_before = Some(ethereum_balance_msg.result);
        //             }
        //         }

        //         println!("Property: {:?}", property);

        //         // Check for all properties in the property descpription folder
        //         property.check().unwrap();
        //     }

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
impl Handler for EthereumSocketHandler {
    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {

        // Try to parse the message into json message
        let message: Value = serde_json::from_str(&msg.to_string()).unwrap();
        self.handle_ethereum(message);
        Ok(())
    }

    fn on_open(&mut self, _shake: ws::Handshake) -> ws::Result<()> {
        println!("Open Websocket for Ethereum");
        let msg = format!("{} opened", self.chain_name);
        self.event_channel.send(msg).unwrap();
        Ok(())
    }
}

#[test]
fn get_topic_ids() {
    use sha3::Digest;

    let event_header = "SendEthToSol(address,string,uint256)";
    utils::get_ethereum_topic_ids(event_header);
}