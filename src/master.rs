use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::vec;
use crate::chunkserver::{*};
use crate::common::{*};


pub trait MasterProcess {
    /// List the files in a directory.
    fn ls(&self, path: &str) -> Vec<String>;
    /// List the file tree for a path prefix (akin to `tree`).
    fn ls_tree(&self, path: &str) -> Vec<String>;
    /// Get the total number of bytes free in the filesystem (disk free).
    fn df(&self) -> u64;
    /// Get the total number of bytes used in the filesystem (disk used).
    fn du(&self) -> u64;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    /// The length of the file in bytes.
    pub length: u64,
    /// The chunks that make up the file.
    pub chunks: Vec<u64>,
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

    /// Chunkserver locations.
    chunk_locations: Vec<ChunkLocation>,
    // chunk_locations: HashMap<u64, Vec<String>>,

    network: Arc<Mutex<NetworkShim>>,
}

struct DiskStats {
    disk_used: u64,
    disk_free: u64,
}

pub struct DatumLocation {
    pub datum_id: [u8; 32],
    pub chunkserver_id: String,
}

struct ChunkLocation {
    chunk_id: u64,
    chunkserver: String,
}

impl MasterServer {
    pub fn new(network: Arc<Mutex<NetworkShim>>, state: MasterServerState) -> MasterServer {
        MasterServer {
            state,
            chunkservers: HashMap::new(),
            network,
            chunk_locations: Vec::new(),
        }
    }

    pub async fn run(&self) {
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

    pub fn get_free_chunkservers(&self, _num_chunks: u64) -> Vec<String> {
        // Get a list of chunkservers that have enough space to store the chunks.
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

    /// Appends to a file path, creating the file if it does not exist.
    pub fn append_file(&mut self, path: &str, datums: Vec<[u8; 32]>, datum_locations: Vec<DatumLocation>, append_length: u64) {
        let mut chunks = vec![];
        let mut chunk_locations = vec![];

        // Get the file entry (or default).
        let mut file = self.state.file_table.get(path).cloned().unwrap_or(File {
            length: 0,
            chunks: vec![],
        });

        for datum in datums {
            // Allocate chunk ID.
            let chunk_id = self.allocate_chunk();

            // Commit the datums at all replicas.
            for loc in &datum_locations {
                if loc.datum_id != datum { continue; }
                
                // Get the chunkserver from the networkshim.
                let chunkserver = self.network
                    .lock().unwrap()
                    .get_node(&loc.chunkserver_id)
                    .unwrap();
                let mut chunkserver = chunkserver.lock().unwrap();

                // Commit the chunk to the chunkserver.
                // TODO err.
                chunkserver.commit_chunk(datum, chunk_id).unwrap();

                // Store the chunk location.
                chunk_locations.push(ChunkLocation {
                    chunk_id,
                    chunkserver: loc.chunkserver_id.clone(),
                });
            }

            // Create the chunk.
            let chunk = Chunk {
                id: chunk_id,
                checksum: 0,
            };
            chunks.push(chunk);
        }

        // Append chunks to file.
        file.chunks.extend(chunks.iter().map(|c| c.id));
        file.length += append_length;

        // Update the file entry.
        self.state.file_table.insert(path.to_string(), file);

        // Update the chunk locations.
        self.chunk_locations.extend(chunk_locations);
    }

    // 
    // Chunkserver API's.
    // 
    
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


}

impl MasterProcess for MasterServer {
    fn ls(&self, path: &str) -> Vec<String> {
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

    fn ls_tree(&self, path: &str) -> Vec<String> {
        let mut result = Vec::new();
        for (file_path, _) in self.state.file_table.iter() {
            if file_path.starts_with(path) {
                result.push(file_path.clone());
            }
        }
        result
    }

    fn df(&self) -> u64 {
        self.compute_stats().disk_free
    }

    fn du(&self) -> u64 {
        self.compute_stats().disk_used
    }


}




