use std::sync::{Mutex, Condvar};

use ws::{Handler, Sender};

pub struct BlockingQueue<T>{
    data: Mutex<Vec<T>>,
    condvar: Condvar,
}

impl<T> BlockingQueue<T>{
    pub fn new() -> Self {
        Self { data: Mutex::new(Vec::new()), condvar: Condvar::new() }
    }

    pub fn push(&self, item: T){
        let mut data = self.data.lock().unwrap();
        data.push(item);
        self.condvar.notify_one();
    }

    pub fn pop(&self) -> T{
        let mut data = self.data.lock().unwrap();
        while data.is_empty() {
            data = self.condvar.wait(data).unwrap();
        }
        data.remove(0)
    }
}


/// An Enum handling whether a transaction is allowed or not 
/// In case of deniance a reason should be given
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum Allowance{
    Allow,
    Deny(Vec<String>)
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Event {
    // Is the Transaction Allowed?
    pub result: Allowance,
    // Which Definition Files have been used to check the result?
    pub checked: Vec<String>,
    // Which Chain?
    pub chain: String,
    // Transaction Hash
    pub transaction_hash: String
}

pub struct HubSocket {
    pub(crate) num_clients: u64,
    pub(crate) sender: Sender,
}

impl Handler for HubSocket {
    fn on_open(&mut self, shake: ws::Handshake) -> ws::Result<()> {
        println!("Client connected: {}", shake.peer_addr.unwrap());
        self.num_clients += 1;
        Ok(())
    }

    fn on_close(&mut self, code: ws::CloseCode, reason: &str) {
        self.num_clients -= 1;
        println!("Client disconnected: {:?}", code);
    }
}
