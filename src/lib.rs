use std::collections::HashMap;
use std::rc::Rc;


use trie_rs::{Trie, TrieBuilder};

struct ChunkserverAddress {
    ip: String,
    port: u16,
}

struct ChunkserverInfo {
    lastSeen: u64,
    ip: String,
    port: u16,
    chunks: Vec<ChunkRef>,
}

struct Lease {
    primary: ChunkserverInfo,
    expiration: u64,
}

struct ChunkInfo {
    lease: Lease,
    primary: ChunkserverInfo,
    replicas: Vec<ChunkserverInfo>,
}

type ChunkRef = u64;

struct File {
    size: u64,
}

struct Chunk {

}

struct MasterProcess {
    chunkToServer: HashMap<ChunkRef, Vec<Rc<ChunkserverInfo>>>,
    chunkservers: Vec<Rc<ChunkserverInfo>>,
    filesystem: HashMap<String, File>,
}

impl MasterProcess {
    fn new() -> MasterProcess {
        MasterProcess {
            chunkToServer: HashMap::new(),
            chunkservers: Vec::new(),
            filesystem: HashMap::new(),
        }
    }

    
}

struct ChunkserverProcess {
    chunkset: Vec<ChunkRef>,
}

fn main() {
    println!("Hello, world!");
}
