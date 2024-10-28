use std::collections::HashMap;
use std::fmt::Error;
use std::sync::{Arc, Mutex};
use std::vec;
use crate::chunkserver::{*};
use crate::common::{*};
use crate::master::{*};
use crate::chunk::{self, *};


pub enum ClientError {
    AppendTooLarge,
    NotEnoughChunkservers,
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

    pub fn read_full(&self, path: &str, network: Arc<Mutex<NetworkShim>>) -> Vec<u8> {
        // 1. Get the file metadata from the master.
        let metadata = self.master.lock().unwrap().stat(path);

        // 2. Begin reading the file in chunks.
        let mut data = vec![];
        let mut offset = 0;

        while offset < metadata.length {
            // The length of the read is the minimum of the remaining file length and the chunk size.
            // We read a single chunk at a time.
            let length = std::cmp::min(metadata.length - offset, CHUNK_SIZE_BYTES as u64);
            
            // 3a. Get the chunk hash and locations from the master.
            let read_info = self.master.lock().unwrap().get_read_infos(path, offset, length).unwrap();

            // Expect only one chunk_read.
            assert_eq!(read_info.chunk_reads.len(), 1);

            // 3b. Read the chunk from the chunkserver.
            let chunk_read = &read_info.chunk_reads[0];
            assert!(1 <= chunk_read.locations.len());
            let location = &chunk_read.locations[0];
            
            let chunk_data = network.lock().unwrap().get_node(&location).unwrap().lock().unwrap().read_chunk(chunk_read.chunk_id).unwrap();

            // 4. Append the chunk data to the file data.
            // If 
            data.extend_from_slice(&chunk_data);
            offset += length;
        }

        data
    }

    // pub fn read(&self, path: &str, offset: u64, length: u64, network: Arc<Mutex<NetworkShim>>) -> Vec<u8> {
    // }

    /// Append data to a file.
    pub fn append(&self, path: &str, data: &[u8], network: Arc<Mutex<NetworkShim>>) -> Result<(), ClientError> {
        let append_length = data.len() as u64;

        println!("writing data size={}", data.len());

        // If the data exceeds 1GB, we return an error.
        if append_length > 1_000_000_000 {
            // TODO in future:
            // - split data 
            return Err(ClientError::AppendTooLarge);
        }

        // 1. Divide the data into chunks.
        let chunks = data_to_chunks(data);
        println!("Appending {} chunks to {path}", chunks.len());

        // 2. Ask master for free chunkservers.
        let replication: u8 = 3;
        let mut free_chunkservers = self.master.lock().unwrap().get_free_chunkservers(chunks.len() as u64, replication);

        if replication < free_chunkservers.len() as u8 {
            return Err(ClientError::NotEnoughChunkservers);
        }

        // 3. Push each chunk to chunkservers with replicas.
        let mut chunk_locations: HashMap<ChunkHash, Vec<String>> = HashMap::new();

        for (i, chunk) in chunks.iter().enumerate() {
            for _ in 0..replication {
                // Select the next chunkserver.
                // Index into the free_chunkservers vector (round-robin).
                let index = i * replication as usize;
                let chunkserver_address = &free_chunkservers[index];

                // Write the chunk to the chunkserver.
                let chunkserver = network.lock().unwrap().get_node(chunkserver_address).unwrap();
                chunkserver.lock().unwrap().push_chunk(&chunk.data).unwrap();

                // Append the chunkserver to the list of chunk locations.
                chunk_locations
                    .entry(chunk.hash)
                    .or_default()
                    .push(chunkserver_address.clone());
            }
        }

        // 4. Commit the chunks at the master.
        let op = AppendOperation {
            file_path: path.to_string(),
            length: append_length,
            chunk_sequence: chunks.iter().map(|chunk| chunk.hash).collect(),
            chunk_locations,
        };
        self.master.lock().unwrap().append_file(op).unwrap();

        Ok(())
    }
}