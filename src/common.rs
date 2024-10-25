use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use sha2::Digest;
use crate::chunkserver::Chunkserver;

pub fn sha256sum(data: &[u8]) -> [u8; 32] {
    let mut hasher = sha2::Sha256::new();
    hasher.update(data);
    let result1 = hasher.finalize();
    let mut result = [0u8; 32];
    result.copy_from_slice(result1.as_slice());
    result
}

pub struct NetworkShim {
    nodes: HashMap<String, Arc<Mutex<Chunkserver>>>,
}

impl NetworkShim {
    pub fn new() -> NetworkShim {
        NetworkShim {
            nodes: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, chunkserver: Arc<Mutex<Chunkserver>>) {
        let cs = chunkserver.lock().unwrap();
        let id = cs.id.clone();
        self.nodes.insert(id, chunkserver.clone());
    }

    pub fn get_node(&self, id: &str) -> Option<Arc<Mutex<Chunkserver>>> {
        match self.nodes.get(id) {
            Some(chunkserver) => Some(chunkserver.clone()),
            None => None,
        }
    }
}