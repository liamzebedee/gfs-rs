use crate::core::*;
use std::collections::HashMap;
use std::rc::Rc;

struct ChunkInfo {
    lease: Lease,
    primary: ChunkserverInfo,
    replicas: Vec<ChunkserverInfo>,
}

struct File {
    size: u64,
    chunks: Vec<ChunkRef>,
}

struct Lease {
    primary: ChunkserverInfo,
    expiration: u64,
}

pub struct MasterProcess {
    chunkToServer: HashMap<ChunkRef, Vec<Rc<ChunkserverInfo>>>,
    chunkservers: Vec<Rc<ChunkserverInfo>>,
    filesystem: HashMap<String, File>,
}

impl MasterProcess {
    pub fn new() -> MasterProcess {
        MasterProcess {
            chunkToServer: HashMap::new(),
            chunkservers: Vec::new(),
            filesystem: HashMap::new(),
        }
    }

    fn write_file() {
        // Get file path.
        // If not exists, create.
        // Map file data to chunks:
        // - size / chunk size = number of chunks
        // - if chunk exists, it's a write
        // - if chunk not exists, it's an allocate + write
        // Allocation routine:
        // - select a chunkserver for primary
        // - select R chunkservers for replicas (where R is configured at the file)
        // Call chunkservers until allocation is complete. Then finalise allocation. Chunks are by default uninitialized. And the file system is served to clients from master, so no risk of returning uninitialized data.
        // Then issue lease for all chunks to client.
        // Client gets a series of chunk leases.
        // Client pushes data to primary chunkservers (which may be different).
        // Each primary pushes data to replicas.
        // Primary awaits ack from replicas.
        // Primary sends ack to client.
        // Client awaits ack from all primaries.
        // Client sends ack to master.
        // Write is finalised at master. The file now "exists" in the trie.
        // Lease is released.
        // While client is waiting, it is renewing lease.
    }
}