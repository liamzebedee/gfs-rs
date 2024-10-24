use std::collections::HashMap;
use std::rc::Rc;
use trie_rs::{Trie, TrieBuilder};

pub mod master;
pub mod chunkserver;
pub mod core;

use crate::master::MasterProcess;
use crate::chunkserver::{ChunkserverProcess};

fn main() {
    // Setup a master.
    let master = MasterProcess::new();

    // Setup 10 workers.
    let mut workers = vec![];
    for _ in 0..10 {
        let worker = ChunkserverProcess::new();
        workers.push(worker);
    }

    // Master maintains list of leases for chunks, list of chunks, list of chunkservers, assignment of chunks to chunkservers.
    // Upon startup, chunkserver locates master, then registers itself. Chunkserver reports the chunks it hosts.
    // The operation log of the master ensures that the state is never inconsistent.
    // When a file is created, the client talks to the master:
}

struct ChunkAllocationRequest {
    /// The ID of the chunk.
    chunk_id: u64,
    /// The version of the chunk.
    version: u64,
    /// The content of the chunk (data).
    content: Vec<u8>,
    /// The hash of the chunk's content.
    content_hash: [u8; 32],
    /// The primary chunkserver for this chunk.
    primary: String,
    /// The secondary chunkservers for this chunk.
    secondaries: Vec<String>,
}

fn create_file(path: &str, size: u64, n_replicas: u64) {
    let file_exists = true;
    let CHUNK_SIZE = 64 * 1024 * 1024; // 64 MB
    
    // Create file.
    if !file_exists {
        let num_chunks = ((size % CHUNK_SIZE) + 1) * n_replicas;
        
        // Determine if we have enough storage to store the file.
        // Lock storage mutex.
        let system_storage_free = 0;
        if system_storage_free < size {
            // Return error "OUT OF SPACE".
            return;
        }
        // Unlock storage mutex.

        // Allocate chunk to chunkservers.
        let mut chunks = vec![];
        for _ in 0..num_chunks {
            // Pick n_replicas chunkservers.
            let mut replicas: Vec<String> = vec![];
            for _ in 0..n_replicas {
                // 
            }

            let primary = replicas[0].clone();
            let secondaries = replicas[1..].to_vec();

            let mut chunk = ChunkAllocationRequest {
                chunk_id: 0,
                version: 0,
                content: vec![],
                content_hash: [0; 32],
                primary: "".to_string(),
                secondaries: vec![],
            };
            chunks.push(chunk);
        }

        // Send chunkserver list to client. Client will write to primary, then primary will write to replicas.
        // Await ack from all chunkservers.
        let on_ack = || {
            // File is written when all chunks are allocated.
            // Chunk allocation request is complete when we receive acks from all replicas.
            // We await a timeout from each replica. If the ack is not received, we emit an allocation failure, and then
            // select another chunkserver.
        };

        // Write file.
        // For each chunkserver, we instruct them to create the chunk.
        
        // Then we create the chunk and chunkserver metadata:
        // - Chunk
        // - Chunkserver allocations

        // Then we create the file entry in our local table:
        // - File
        // - Chunks

        // And return done to the client.
    }
}


// Failure modes:
// - when a chunkserver fails
// - when a chunkserver starts up and has stale data
// - 

struct ChunkInfo {
    id: u64,
    version: u64,
    hash: [u8; 32],
}

fn on_chunkserver_register(storage: u64, storage_free: u64, chunkset: Vec<ChunkInfo>) {
    // Get the chunk Set.
    // Update any stale chunks.
    // Then add to metadata for system, as a secondary. 
    // If there are no primaries, add as primary? 
}

fn on_chunkserver_heartbeat() {
    // If chunkserver misses heartbeat, we have to mark as dead and recompute the state.
}

// We need to grant a lease to the primary in order for it to handle writes
// When the primary dies, we need to reassign the lease to another chunkserver.
// When the master receives a heartbeat, we renew the lease.

// Client RPC (to master):
// - read(path, offset, len) -> (chunkserver locations[])
    // - translate into chunk indices
    // - lookup servers for each chunk
    // - return servers
// - write(path, offset, len) -> (chunkserver locations[])
    // - translate into chunk indices
    // - lookup servers for each chunk
    // - return servers

// Client RPC (to chunkserver):
// - read(chunk_id, offset, len) -> (data)

// Write flow:
// - master issues/grants lease to primary
// - client pushes data to replicas
// - client sends write req to primary, with acks from replicas
// - primary forwards write req to replicas
// - primary awaits all replicas reply
// - primary commits write

// How does the master know the current version of a chunk?
// The master keeps track of the chunk primary, which is the replica which serialises writes.
// if the primary fails, then the 

// Heartbeat interval per chunkserver: 30s
// Heartbeat contains all chunks stored