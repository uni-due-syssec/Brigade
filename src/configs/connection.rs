use std::{mem::MaybeUninit, sync::Once};

use ws::Sender;

#[derive(Debug, Clone)]
pub struct ConnectionList {
    pub connections: Vec<(String, Sender)>,
}

impl ConnectionList {
    pub fn new() -> Self {
        // println!("Initializing Connection List");
        Self {
            connections: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.connections.len()
    }

    pub fn insert(&mut self, name: String, sender: Sender) -> Option<&Sender> {
        // println!("Adding Connection: {}", name);
        if let Some(o) = self.get_id(name.as_str()) {
            // println!("Connection already exists: {}", name);
            return Some(&self.connections[o].1);
        }

        self.connections.push((name, sender));
        // Return a reference to the newly inserted sender
        Some(&self.connections.last()?.1)
    }

    pub fn remove(&mut self, name: &str) {
        self.connections.retain(|(n, _)| n != name);
    }

    pub fn get(&self, name: &str) -> Option<&Sender> {
        let out = self
            .connections
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, s)| s);
        // println!("Connection: {:?}", out);
        out
    }

    pub fn get_id(&self, name: &str) -> Option<usize> {
        self.connections.iter().position(|(n, _)| n == name)
    }
}

pub fn get_established_connections() -> &'static mut ConnectionList {
    static mut MAYBE: MaybeUninit<ConnectionList> = MaybeUninit::uninit();
    static ONLY: std::sync::Once = Once::new();

    unsafe {
        ONLY.call_once(|| {
            let con_list = ConnectionList::new();
            MAYBE.write(con_list);
        });
        MAYBE.assume_init_mut()
    }
}
