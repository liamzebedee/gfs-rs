use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::vec;
use crate::chunk::ChunkHash;
use crate::chunkserver::{*};
use crate::common::{*};
use crate::chunk::{*};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MasterError {
    FileNotFound,
    EndOfFile,
    ChunkNotFound,
    ChunkserverNotFound,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct File {
    /// The length of the file in bytes.
    pub length: u64,
    /// The chunks that make up the file.
    pub chunks: Vec<u64>,
}

pub struct StatInfo {
    /// The length of the file in bytes.
    pub length: u64,
}

#[allow(dead_code)]
struct ChunkserverInfo {
    id: String,
    last_seen: u64,
    ip: String,
    port: u16,
    chunks: Vec<u64>,
    disk_used: u64,
    disk_free: u64,
}


/// The persistent state of the master server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterServerState {
    file_table: HashMap<String, File>,
    chunk_counter: u64,
}

impl MasterServerState {
    pub fn new() -> MasterServerState {
        MasterServerState {
            file_table: HashMap::new(),
            chunk_counter: 0,
        }
    }

    pub fn from_file(path: PathBuf) -> MasterServerState {
        // Load the state from a file.
        let file = std::fs::read_to_string(path).unwrap();
        let state: MasterServerState = serde_json::from_str(&file).unwrap();
        state
    }

    pub fn to_file(&self, path: PathBuf) {
        // Save the state to a file.
        let file = serde_json::to_string(self).unwrap();
        std::fs::write(path, file).unwrap();
    }
}

pub struct MasterServer {
    state: MasterServerState,

    // Ephermal state.
    chunkservers: HashMap<String, ChunkserverInfo>,
    chunk_locations: HashMap<u64, Vec<String>>,

    network: Arc<Mutex<NetworkShim>>,
}

struct DiskStats {
    disk_used: u64,
    disk_free: u64,
}


struct ChunkLocation {
    chunk_id: u64,
    chunkserver: String,
}

pub struct AppendOperation {
    /// The file path to append to.
    pub file_path: String,

    /// The sequence of chunk hashes.
    pub chunk_sequence: Vec<ChunkHash>,

    /// The locations of the chunks.
    pub chunk_locations: HashMap<ChunkHash, Vec<String>>,
    
    /// The length of the data to append in bytes.
    pub length: u64,
}

pub struct ChunkRead {
    pub chunk_id: u64,
    pub locations: Vec<String>,
}

pub struct ReadOperationInfo {
    pub path: String,
    pub offset: u64,
    pub length: u64,
    pub chunk_reads: Vec<ChunkRead>,
}

impl MasterServer {
    pub fn new(network: Arc<Mutex<NetworkShim>>, state: MasterServerState) -> MasterServer {
        MasterServer {
            state,
            chunkservers: HashMap::new(),
            network,
            chunk_locations: HashMap::new(),
        }
    }

    pub fn run(&self) {
        // Run the master server.
    }

    fn compute_stats(&self) -> DiskStats {
        let mut disk_used = 0;
        let mut disk_free = 0;

        self.chunkservers.iter().for_each(|(_, chunkserver_info)| {
            disk_used += chunkserver_info.disk_used;
            disk_free += chunkserver_info.disk_free;
        });

        DiskStats { disk_used, disk_free }
    }

    /// Get a list of chunkservers that have enough free storage space to store the chunks.
    pub fn get_free_chunkservers(&self, _num_chunks: u64, replication_factor: u8) -> Vec<String> {
        let mut chunkservers = self.chunkservers.values().collect::<Vec<&ChunkserverInfo>>();
        // Sort by disk free space and then map onto id
        chunkservers.sort_by(|a, b| a.disk_free.cmp(&b.disk_free));
        chunkservers.into_iter().map(|x| x.id.clone()).collect()
    }

    fn allocate_chunk(&mut self) -> u64 {
        let chunk_id = self.state.chunk_counter;
        self.state.chunk_counter += 1;
        chunk_id
    }


    ///
    /// Chunkserver file API's.
    /// 

    /// Appends to a file path, creating the file if it does not exist.
    pub fn append_file(&mut self, op: AppendOperation) -> Result<(), String> {
        // 1. Allocate chunk ID for each chunk.
        let hash_to_chunk_id: HashMap<ChunkHash, u64> = op.chunk_sequence.iter().map(|hash| {
            let chunk_id = self.allocate_chunk();
            (hash.clone(), chunk_id)
        }).collect();

        // 2. Commit each chunk.
        let mut committed_chunk_locations: HashMap<u64, Vec<String>> = HashMap::new();

        // For each chunk and its locations:
        for (chunk_hash, chunk_locations) in op.chunk_locations.iter() {
            // Get the chunk ID.
            let chunk_id = hash_to_chunk_id[chunk_hash];

            // For each location.
            for (i, chunk_location) in chunk_locations.iter().enumerate() {
                // 1. Get the chunkserver.
                let chunkserver = self.network.lock().unwrap().get_node(chunk_location).unwrap();
                // TODO handle chunkserver not found in real implementation.
                
                // 2. Commit the chunk.
                let res = chunkserver.lock().unwrap().commit_chunk(*chunk_hash, chunk_id);
                
                // If failed, just ignore.
                if res.is_err() {
                    println!("[master] failed to commit chunk {} to {}; skipping", chunk_id, chunk_location);
                    continue;
                }

                // 3. Store the chunk location.
                committed_chunk_locations
                    .entry(chunk_id)
                    .or_default()
                    .push(chunk_location.clone());
            }
        }

        // 3. Update the file entry.
        let file = self.state.file_table.entry(op.file_path.clone()).or_default();
        file.chunks.extend(committed_chunk_locations.keys());
        file.length += op.length;
        println!("[master] append {} bytes={} chunks={}", op.file_path, op.length, committed_chunk_locations.keys().len());

        // 4. Update the chunk locations.
        // TODO check what Rust does with the iterator here.
        self.chunk_locations.extend(committed_chunk_locations);

        Ok(())
    }

    /// 
    /// Chunkserver control API's.
    /// 
    
    /// Receive a heartbeat from a chunkserver.
    pub fn receive_heartbeat(&mut self, chunkserver_id: String, disk_used: u64, disk_free: u64) {
        println!("Received heartbeat from chunkserver: {chunkserver_id}");

        // Update the last seen time for the chunkserver.
        if let Some(chunkserver_info) = self.chunkservers.get_mut(&chunkserver_id) {
            chunkserver_info.last_seen = 0;
        } else {
            // Add the chunkserver to the list of chunkserver's.
            self.chunkservers.insert(chunkserver_id.clone(), ChunkserverInfo {
                id: chunkserver_id.clone(),
                last_seen: 0,
                ip: String::new(),
                port: 0,
                chunks: Vec::new(),
                disk_used,
                disk_free,
            });
        }
    }


    ///
    /// Client API's.
    /// 

    /// List the files in a directory.
    pub fn ls(&self, path: &str) -> Vec<String> {
        let mut result = Vec::new();
        let base_path = Path::new(path);
        
        for (file_path, _) in self.state.file_table.iter() {
            let file_path = Path::new(file_path);
            
            if file_path.starts_with(base_path) {
                // Check that file is directly inside the directory, not in subdirectories
                if let Some(relative_path) = file_path.strip_prefix(base_path).ok() {
                    if relative_path.parent().is_none() {
                        result.push(file_path.to_string_lossy().into_owned());
                    }
                }
            }
        }
        
        result
    }

    /// List the file tree for a path prefix (akin to `tree`).
    pub fn ls_tree(&self, path: &str) -> Vec<String> {
        let mut result = Vec::new();
        for (file_path, _) in self.state.file_table.iter() {
            if file_path.starts_with(path) {
                result.push(file_path.clone());
            }
        }
        result
    }

    /// Get the total number of bytes free in the filesystem (disk free).
    pub fn df(&self) -> u64 {
        self.compute_stats().disk_free
    }

    /// Get the total number of bytes used in the filesystem (disk used).
    pub fn du(&self) -> u64 {
        self.compute_stats().disk_used
    }

    /// Get the metadata for a file.
    pub fn stat(&self, path: &str) -> StatInfo {
        let file = self.state.file_table.get(path).unwrap();
        StatInfo { length: file.length }
    }

    /// Get chunks and their locations for a read operation.
    pub fn get_read_infos(&self, path: &str, offset: u64, length: u64) -> Result<ReadOperationInfo, MasterError> {
        println!("[master] get_read_infos path={} offset={} length={}", path, offset, length);

        let Some(file) = self.state.file_table.get(path) else { return Err(MasterError::FileNotFound) };
        let offset_chunk = offset / CHUNK_SIZE_BYTES as u64;
        let end_chunk = (offset + length) / CHUNK_SIZE_BYTES as u64;

        // The offset chunk is the first chunk we need to read.
        // If the offset exceeds the file length, return an EOF error.
        if file.length < offset {
            return Err(MasterError::EndOfFile);
        }

        // The end chunk is the last chunk we need to read.
        // If the end chunk exceeds the file length, truncate it.
        let end_chunk = std::cmp::min(end_chunk, (file.length as f64 / CHUNK_SIZE_BYTES as f64).ceil() as u64);

        let mut chunk_reads = vec![];
        // For each chunk, get the chunk ID and locations.
        for i in offset_chunk..=end_chunk {
            let chunk_id = file.chunks[i as usize];
            let locations = self.chunk_locations.get(&chunk_id).unwrap();
            chunk_reads.push(ChunkRead { chunk_id, locations: locations.clone() });
        }

        Ok(ReadOperationInfo { path: path.to_string(), offset, length, chunk_reads })
    }

}
