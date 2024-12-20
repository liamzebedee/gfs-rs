use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::vec;
use sha2::Digest;
use lru::LruCache;
use std::num::NonZeroUsize;
use crate::master::MasterServer;
use crate::common::{*};
use crate::chunk::{*};

pub struct Chunkserver {
    master: Arc<Mutex<MasterServer>>,
    pub id: String,
    disk_allocation: u64,

    /// The LRU cache for chunks.
    lru_cache: LruCache<[u8; 32], Vec<u8>>,

    /// The storage for the chunkserver.
    storage: ChunkserverStorage,
}


#[derive(Debug, Clone)]
pub enum ChunkserverError {
    InvalidChunkLength,
    ChunkNotFound,
}

pub struct Chunk {
    pub id: u64,
    pub len: u64,
    pub checksum: u32,
}



pub struct ChunkserverStorage {
    // The path to the chunkserver storage directory.
    storage_dir: PathBuf,

    // List of chunks.
    chunks: Vec<Chunk>,
}


impl ChunkserverStorage {
    pub fn new(storage_dir: PathBuf) -> ChunkserverStorage {
        use crc32fast::Hasher;

        // If directory does not exist, create it.
        if !storage_dir.exists() {
            std::fs::create_dir_all(&storage_dir).unwrap();
        }

        let mut chunks = Vec::new();

        // List all files.
        let files = std::fs::read_dir(&storage_dir).unwrap();
        for file in files {
            let file = file.unwrap();
            let name = file.file_name();
            
            // if name begins with ch
            if name.to_str().unwrap().starts_with("ch") {
                // parse the chunk ID
                let chunk_id = name.to_str().unwrap().split_at(2).1.parse::<u64>().unwrap();
                // compute the checksum
                // load chunk data
                let mut hasher = Hasher::new();
                // ensure file is CHUNK SIZE bytes
                if file.metadata().unwrap().len() != CHUNK_SIZE_BYTES as u64 {
                    continue;
                }
                let data = std::fs::read(file.path()).unwrap();
                let checksum = crc32fast::hash(&data);
                let chunk = Chunk { id: chunk_id, len: data.len() as u64, checksum };

                println!("Chunk: {chunk_id} {checksum}");
                // add to chunks
                chunks.push(chunk);
            }
        }
        ChunkserverStorage { storage_dir, chunks }
    }

    pub fn write_chunk(&mut self, chunk_id: u64, data: &[u8]) {
        // Write the data to disk in the storage directory.
        let chunk_path = self.storage_dir.join(format!("ch{chunk_id}"));
        std::fs::write(chunk_path, data).unwrap();

        // Compute checksum.
        let checksum = crc32fast::hash(data);

        // Add the chunk to the chunk list.
        self.chunks.push(Chunk { id: chunk_id, len: data.len() as u64, checksum: checksum });
    }

}

impl Chunkserver {
    pub fn new(master: Arc<Mutex<MasterServer>>, id: String, disk_allocation: u64, storage: ChunkserverStorage) -> Chunkserver {
        Chunkserver { 
            master, 
            id,
            disk_allocation,
            lru_cache: LruCache::new(NonZeroUsize::new(20).unwrap()),
            storage,
        }
    }

    pub fn run(&self) {
        // Run the chunkserver.
        self.master.lock().unwrap().receive_heartbeat(
            self.id.clone(),
            0,
            self.disk_allocation,
        );
    }
    
    /// Receive a chunk datum pushed by a client into the LRU cache.
    pub fn push_chunk(&mut self, data: &[u8]) -> Result<(), ChunkserverError> {
        if data.len() != CHUNK_SIZE_BYTES {
            return Err(ChunkserverError::InvalidChunkLength);
        }

        // Compute the chunk datum ID (SHA256).
        let chunk_hash = sha256sum(data);

        // Insert the chunk into the LRU cache.
        self.lru_cache.put(chunk_hash, data.to_vec());

        Ok(())
    }

    /// Commit a datum from LRU cache to disk.
    /// This is called by the master server.
    pub fn commit_chunk(&mut self, chunk_hash: ChunkHash, chunk_id: u64) -> Result<(), ChunkserverError> {
        // Get the value from LRU, if it is missing return error.
        let value_res = self.lru_cache.get(&chunk_hash);
        if value_res.is_none() {
            return Err(ChunkserverError::ChunkNotFound);
        }

        // Store a chunk on disk with the ID from the master.
        // Write the data to disk in the storage directory.
        self.storage.write_chunk(chunk_id, value_res.unwrap());

        // Remove the datum from the LRU cache.
        self.lru_cache.pop(&chunk_hash);

        Ok(())
    }

    /// Read a chunk from the storage.
    /// This is called by clients.
    pub fn read_chunk(&self, chunk_id: u64) -> Result<Vec<u8>, ChunkserverError> {
        // Find the chunk in the storage.
        let chunk = self.storage.chunks.iter().find(|c| c.id == chunk_id);
        if chunk.is_none() {
            return Err(ChunkserverError::ChunkNotFound);
        }

        // Read the chunk from disk.
        let chunk_path = self.storage.storage_dir.join(format!("ch{chunk_id}"));
        let data = std::fs::read(chunk_path).unwrap();

        Ok(data)
    }
}