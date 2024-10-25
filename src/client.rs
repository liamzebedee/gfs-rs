use std::sync::{Arc, Mutex};
use std::vec;
use crate::chunkserver::{*};
use crate::common::{*};
use crate::master::{MasterServer,MasterProcess,DatumLocation};


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

    pub fn read(&self, path: &str, offset: u64, length: u64, network: Arc<Mutex<NetworkShim>>) -> Vec<u8> {
        vec![]
    }

    // Things to fix:
    // 1. append -> (fill, allocate-append)
    // do we want writes to be atomic? I don't know. I think yes. we want to share big files (ML datasets). which means
    // putting in a write request to the master with that amount. the allocator can then allocate that size (or ask more storage to come online). and then after that happens, we write the file.
    // what does writing the file do? I have two ideas for this system:
    // 1. like GFS mainline, basically we write chunks to chunkservers then commit them at master
    // 2. like bittorrent/ipfs, we discover nodes that host datasets and then build a useful index so we can read them quickly.
    // the problem with IPFS is that it's slow. the metadata is slow to access and the actual servers are slow to download from (client issue). and the sync protocol is horribly inefficient
    // 
    // let's just do mainline to be simple, and then we'll fix the rest later??
    // the thing I want to enable:
    // - people can onboard big datasets onto goliath
    // the way they do that
    // first they mirror it locally
    // then they contact goliath master and it indexes them
    // and they can optionally pay to mirror the dataset among a network of nodes
    // where they pay for it
    // 
    // so okay two flows:
    // 1. upload data to dataset. involves registering metadata (file names, paths) and then sharing the data for replication.
    // 2. simply have append-only thing. this only allows a certain flow of the data into the system at once. copying would be slow in a P2P model. but you could do it.
    // 
    // it really depends on customer needs. basically people want to be able to import their data easily into the system. writing is hard in the current model.
    // on the converse, this append-only model works well for systems that are like bigtable. it enables you to build reliable things out of it.
    // bigtable isn't a distributable thing yet though. I think a simple extension to ethereum as an extensible blockchain history journal would be really good. then you could sync more quickly.
    // I think that basically having a share drive where you can mirror files locally, they exist in the same shared namespace to everyone, would be quite useful
    // ipfs went some way in doing this but the client was really slow. I think the GFS single-master metadata approach works a bit better though obviously it's constrained, it's a solution
    // 
    // so I think there are 3 markets:
    // 1. blockchain history storage journal
    // 2. global share drive
    // 
    // in the global share drive case, you want to basically progressively upload files into the drive, and mirror subsets taht you want (paths). it's a cryptonetwork with data sharing built in.
    // in the history example, you basically want to use it as a third-party network. you don't care about mirroring so much as you don't store any state yourself. however you might want to cache certain aspects of the storage log or files themselves.
    // 
    // 
    // okay so there are two versions of this:
    // - history
    // - global share drive
    // 
    // what are the features?
    // - atomic reads
    // - atomic writes of chunks
    // - atomic writes of large files
    // - atomic append
    // - controllable replication
    // 
    // 
    // and the diferences are:
    // - in the history example, we use it as a storage journal. nodes append data to the journal in files. the appends can take some time, since we are uploading data to chunkservers. it's important we maintain a strict ordering per-file but otherwise we don't mind. 
    // really the metadata is assembled at the master, so long as each chunkserver can store some chunks, then we can create the file and be done with it.
    // we can publish the file manifest at the master
    // imagine we had a phenoemenon of a single writer
    // we can start the system in this state where there is a single file manifest and we import data in
    // and the writer
    // 
    // I think we want to 

    /// Append data to a file.
    pub fn append(&self, path: &str, data: &[u8], network: Arc<Mutex<NetworkShim>>) {
        let append_length = data.len() as u64;

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
            let chunkserver = network.lock().unwrap().get_node(chunkserver_id.as_str()).unwrap();
            chunkserver.lock().unwrap().push_chunk(data);

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
            append_length,
        );
    }
}