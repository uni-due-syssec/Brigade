use std::sync::mpsc::Sender;

use ethnum::AsU256;
use reqwest::blocking::Client;
use serde_json::{json, Value};
use ws::Handler;

use crate::get_variable_map_instance;
use crate::utils::get_startup_time;
use crate::VarValues;
use crate::{message_formats::ethereum_message::*, properties::Properties, set_var, utils};

use crate::message_formats::solana_message::*;

/// Solana Websocket Handler
pub struct SolanaSocketHandler {
    // State of the Client
    pub(crate) chain_name: String,
    pub(crate) properties: Vec<Properties>,
    pub(crate) event_channel: Sender<Properties>,
    request_url: String,
}

impl SolanaSocketHandler {
    pub fn new(
        properties: Vec<Properties>,
        event_channel: Sender<Properties>,
        request_url: String,
    ) -> Self {
        Self {
            chain_name: "solana".to_string(),
            properties,
            event_channel,
            request_url,
        }
    }

    pub fn handle(&mut self, message: Value) {
        // println!("Message: {:?}", serde_json::to_string_pretty(&message).unwrap());

        // Interpret Message if Event: resume else quit
        if let Ok(msg) = serde_json::from_value::<LogMessage>(message) {
            // Check which event occured
            let event_string = msg
                .params
                .result
                .value
                .logs
                .iter()
                .find(|x| x.to_lowercase().contains("event:"));
            match event_string {
                Some(event_content) => {
                    // println!("Event: {}", event_string.unwrap());
                    self.properties.push(Properties::new());
                    let index = self.properties.len() - 1;

                    // Event was found
                    let event = &event_content[13..];
                    self.properties[index].occured_event = Some(event.to_string());

                    // concat logs as event data
                    let event_data = msg.params.result.value.logs.concat();
                    set_var!("solana_event_data", event_data);

                    // transaction signature
                    let transaction_signature = msg.params.result.value.signature.clone();
                    self.properties[index].transaction_hash = Some(transaction_signature.clone());

                    // Get Transaction
                    let get_transaction = json!({
                        "jsonrpc": "2.0",
                        "method": "getTransaction",
                        "params": [transaction_signature, {"encoding": "jsonParsed","maxSupportedTransactionVersion":0}],
                        "id": 1
                    }).to_string();

                    // Build HTTP Post for Transaction Data
                    let client = Client::new();
                    let request_body: Value =
                        serde_json::from_str(get_transaction.as_str()).unwrap();
                    // println!("Request Body: {}", serde_json::to_string_pretty(&request_body).unwrap());
                    let res = client
                        .post(self.request_url.clone())
                        .json(&request_body)
                        .send()
                        .unwrap();

                    let body = res.text().unwrap();
                    if let Ok(transaction_msg) =
                        serde_json::from_str::<TransactionMessage>(&body.as_str())
                    {
                        // println!("Transaction Message: {}", serde_json::to_string_pretty(&transaction_msg).unwrap());

                        // Get Slot
                        self.properties[index].block_number =
                            Some(transaction_msg.result.slot.clone().as_u256());

                        // Find Payer
                        let payer = find_payer(&transaction_msg);
                        match payer {
                            Some(idx) => {
                                self.properties[index].payer_address = Some(
                                    transaction_msg.result.transaction.message.account_keys[idx]
                                        .pubkey
                                        .clone(),
                                );
                                self.properties[index].payer_balance_before = Some(
                                    transaction_msg.result.meta.pre_balances[idx]
                                        .clone()
                                        .as_u256(),
                                );
                                self.properties[index].payer_balance_after = Some(
                                    transaction_msg.result.meta.post_balances[idx]
                                        .clone()
                                        .as_u256(),
                                );
                            }
                            None => {
                                self.properties[index].payer_address = None;
                                self.properties[index].payer_balance_before = None;
                                self.properties[index].payer_balance_after = None;
                            }
                        }

                        // Solana has no value like Ethereum
                        self.properties[index].value = Some(0.as_u256());

                        self.properties[index].src_chain = Some(self.chain_name.clone());

                        // Send the Event to the Event Channel
                        self.event_channel
                            .send(self.properties[index].clone())
                            .unwrap();
                    } else {
                        println!("Wrong Transaction Message Format");
                        // println!("Body: {}", serde_json::to_string_pretty(&body).unwrap());
                        return;
                    }
                }
                None => {
                    // Quitting handling as no Event was found
                    println!("No Event");
                    return;
                }
            }
        }
    }
}

/// Here the WebSocket Handles the basic workflow
impl Handler for SolanaSocketHandler {
    fn on_message(&mut self, msg: ws::Message) -> ws::Result<()> {
        // Try to parse the message into json message
        let message: Value = serde_json::from_str(&msg.to_string()).unwrap();
        self.handle(message);
        Ok(())
    }

    fn on_open(&mut self, _shake: ws::Handshake) -> ws::Result<()> {
        println!("Open Websocket for Solana");
        let msg = format!("{} opened", self.chain_name);
        Ok(())
    }
}

pub fn find_payer(transaction: &TransactionMessage) -> Option<usize> {
    let num_signer = transaction
        .result
        .transaction
        .message
        .account_keys
        .iter()
        .map(|x| x.signer == true)
        .count();
    if num_signer == 1 {
        for count in 0..transaction.result.transaction.message.account_keys.len() {
            if transaction.result.transaction.message.account_keys[count].signer == true {
                return Some(count);
            }
        }
    }

    let post_balances = transaction.result.meta.post_balances.clone();
    let pre_balances = transaction.result.meta.pre_balances.clone();
    let fee = transaction.result.meta.fee;

    for index in 0..post_balances.len() {
        if post_balances[index] - pre_balances[index] == fee {
            return Some(index);
        }
    }
    None
}
