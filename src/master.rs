use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::vec;
use lru::LruCache;
use std::num::NonZeroUsize;

fn sha256sum(data: &[u8]) -> [u8; 32] {
    use sha2::Digest;
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

pub struct MasterServer {
    // Master state.
    file_table: HashMap<String, File>,

    // Ephermal state.
    chunkservers: HashMap<String, ChunkserverInfo>,

    // Chunk counter.
    chunk_counter: u64,

    /// Chunkserver locations.
    chunk_locations: HashMap<u64, Vec<String>>,

    network: Arc<Mutex<NetworkShim>>,
}

struct DiskStats {
    disk_used: u64,
    disk_free: u64,
}

/// A datum is a chunk stored in the LRU cache before it is committed to disk.
/// It is identified by a SHA256 hash, it does not have a chunk ID.
struct Datum {
}

struct DatumLocation {
    datum_id: [u8; 32],
    chunkserver_id: String,
}

impl MasterServer {
    pub fn new(network: Arc<Mutex<NetworkShim>>) -> MasterServer {
        MasterServer {
            file_table: HashMap::new(),
            chunkservers: HashMap::new(),
            network,
            chunk_counter: 0,
            chunk_locations: HashMap::new(),
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
        let chunk_id = self.chunk_counter;
        self.chunk_counter += 1;
        chunk_id
    }

    /// Appends to a file path, creating the file if it does not exist.
    fn append_file(&mut self, path: &str, datums: Vec<[u8; 32]>, datum_locations: Vec<DatumLocation>, file_length: u64) {
        let mut chunks = vec![];

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
            }

            // Create the chunk.
            let chunk = Chunk {
                id: chunk_id,
                checksum: 0,
            };
            chunks.push(chunk);
        }

        // Now create file entry.
        let file = File {
            length: file_length,
            chunks: chunks.iter().map(|c| c.id).collect(),
        };

        // Then it will create the file entry with the chunk locations and commit.
        self.file_table.insert(path.to_string(), file);
    }

    // 
    // Chunkserver API's.
    // 
    
    /// Receive a heartbeat from a chunkserver.
    fn receive_heartbeat(&mut self, chunkserver_id: String, disk_used: u64, disk_free: u64) {
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
        
        for (file_path, _) in self.file_table.iter() {
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
        for (file_path, _) in self.file_table.iter() {
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


pub struct Chunkserver {
    master: Arc<Mutex<MasterServer>>,
    pub id: String,
    disk_allocation: u64,

    /// The LRU cache for chunks.
    lru_cache: LruCache<[u8; 32], Vec<u8>>,

    /// The storage for the chunkserver.
    storage: ChunkserverStorage,
}

// Chunk size is 1KB.
const CHUNK_SIZE_BYTES: usize = 1024;

struct Chunk {
    id: u64,
    checksum: u32,
}


pub struct ChunkserverStorage {
    // The path to the chunkserver storage directory.
    storage_dir: PathBuf,

    // List of chunks.
    chunks: Vec<Chunk>,
}

#[derive(Debug, Clone)]
struct NoDatumError;


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
                let chunk = Chunk { id: chunk_id, checksum };

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
        self.chunks.push(Chunk { id: chunk_id, checksum: checksum });
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
    pub fn push_chunk(&mut self, data: &[u8]) -> [u8; 32] {
        use sha2::Digest;
        
        // Compute the chunk datum ID (SHA256).
        let datum_id = sha256sum(data);
        self.lru_cache.put(datum_id, data.to_vec());

        datum_id
    }

    /// Commit a datum from LRU cache to disk.
    /// This is called by the master server.
    pub fn commit_chunk(&mut self, datum_id: [u8; 32], chunk_id: u64) -> Result<(), NoDatumError> {
        // Get the value from LRU, if it is missing return error.
        let value_res = self.lru_cache.get(&datum_id);
        if value_res.is_none() {
            return Err(NoDatumError);
        }

        // Store a chunk on disk with the ID from the master.
        // Write the data to disk in the storage directory.
        self.storage.write_chunk(chunk_id, value_res.unwrap());

        // Remove the datum from the LRU cache.
        self.lru_cache.pop(&datum_id);

        Ok(())
    }
}


pub struct Client {
    master: Arc<Mutex<MasterServer>>,
}

impl Client {
    pub fn new(master: Arc<Mutex<MasterServer>>) -> Client {
        Client { master }
    }

    /// Get the total number of bytes free in the filesystem (disk free).
    pub fn df(&self) -> u64 {
        self.master.lock().unwrap().df()
    }

    /// Get the total number of bytes used in the filesystem (disk used).
    pub fn du(&self) -> u64 {
        self.master.lock().unwrap().du()
    }

    /// List the files in a directory.
    pub fn ls(&self, path: &str) -> Vec<String> {
        self.master.lock().unwrap().ls(path)
    }

    /// List the file tree for a path prefix (akin to `tree`).
    pub fn ls_tree(&self, path: &str) -> Vec<String> {
        self.master.lock().unwrap().ls_tree(path)
    }

    pub fn append(&self, path: &str, data: &[u8], network: Arc<Mutex<NetworkShim>>) {
        // First divide the data into datums.
        let num_chunks = (data.len() as f64 / CHUNK_SIZE_BYTES as f64).ceil() as u64;
        println!("Appending {num_chunks} chunks to {path}");
        let mut datums = vec![];

        // Compute the list of datum ID's (sha2 hashes).
        for i in 0..num_chunks {
            let start = i as usize * CHUNK_SIZE_BYTES;
            let end = std::cmp::min((i + 1) as usize * CHUNK_SIZE_BYTES, data.len());
            let chunk = &data[start..end];
            let datum_id = sha256sum(chunk);
            datums.push((datum_id, chunk));
        }

        // Then ask master for a set of chunkservers free to store these chunks at a certain replication level.
        let mut free_chunkservers = self.master.lock().unwrap().get_free_chunkservers(num_chunks);

        // Then push data to each chunkserver.
        let mut datum_locations = vec![];

        let datum_ids: Vec<[u8; 32]> = datums.iter().map(|(datum_id, _)| *datum_id).collect();
        
        // Round-robin the datums to the chunkservers.
        for (datum_id, data) in datums {
            // Get chunkserver.
            let chunkserver_id = &free_chunkservers.pop().unwrap();

            // Push chunk data.
            println!("Pushing data to chunkserver {chunkserver_id}");
            let node = network.lock().unwrap().get_node(chunkserver_id.as_str()).unwrap();
            let datum_id = node.lock().unwrap().push_chunk(data);

            // Store location.
            let loc = DatumLocation {
                datum_id: datum_id,
                chunkserver_id: chunkserver_id.to_string(),
            };
            datum_locations.push(loc);

            // Add the chunkserver back to the list.
            free_chunkservers.push(chunkserver_id.to_string());

            // TODO:
            // - this doesn't actually round robin properly. But for example's sake it's fine.
        }

        // Then ask master to create the file and commit.
        self.master.lock().unwrap().append_file(
            path, 
            datum_ids,
            datum_locations,
            data.len() as u64,
        );
    }
}