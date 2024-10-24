use gfs::master::MasterServer;
use gfs::master::Client;
use gfs::master::Chunkserver;
use gfs::master::ChunkserverStorage;
use gfs::master::NetworkShim;
use byte_unit::Byte;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use tokio::sync::Mutex as TMutex;


#[tokio::main]
async fn main() {
    let network = Arc::new(Mutex::new(NetworkShim::new()));

    // Setup master.
    println!("Creating master.\n");
    let master = Arc::new(Mutex::new(MasterServer::new(network.clone())));

    // Setup client.
    println!("Creating client.\n");
    let client: Client = Client::new(master.clone());
    
    // Get a directory listing.
    println!("ls /"); client.ls("/").iter().for_each(|x| println!("{}", x));
    println!("ls /files/"); client.ls("/files/").iter().for_each(|x| println!("{}", x));

    // Get disk usage.
    println!("df"); println!("disk free: {:#}", Byte::from_u64(client.df()));
    println!("du"); println!("disk used: {:#}", Byte::from_u64(client.du()));

    // Setup chunkserver 1-N.
    let n_chunkservers = 3;

    // What do we want? 
    // We want the chunkserver to be mutable and accessed by:
    // - the master
    // - the client
    // - the chunkserver itself
    // which means 3 mutable references to the same object.
    // We can use a TMutex to wrap the chunkserver and allow async access.
    // We can use an Arc to share the TMutex between the master, client, and chunkserver.
    // but this means changing a lot of code to use async
    // simply because tokio is async and we need to use tokio::spawn to run the chunkserver.
    // is there another way? to use a sync mutex and spawn a thread?
    // yes there is.
    // just use a std::thread::spawn to spawn a thread. no async needed lol.

    // how do we share the network?
    // who uses network shim?
    // - master (to locate chunkserver)
    // - client (to locate chunkserver)
    // - main (to add chunkserver)
    // is it mutable or read-only?
    // - master will only read values
    // - client will only read values
    // - main will only write values
    

    for i in 0..n_chunkservers {
        println!("Creating chunkserver {}.\n", i);
        // data path is relative ./data/chunkserver-{i}
        let storage_dir = PathBuf::from(format!("./data/chunkserver-{i}"));
        let storage = ChunkserverStorage::new(storage_dir);
        let chunkserver = Arc::new(Mutex::new(Chunkserver::new(master.clone(), format!("chunkserver-{i}"), 1024, storage)));
        let cs2 = chunkserver.clone();

        // Start the chunkserver.
        // tokio::spawn(async move {
        //     chunkserver.lock().await.run();            
        // });
        std::thread::spawn(move || {
            chunkserver.lock().unwrap().run();
        });

        // Register the chunkserver with the master.
        network.clone().lock().unwrap().add_node(cs2);
    }

    // Wait for all chunkservers to start.

    // Run the master server independently.
    // tokio::spawn(async move {
    //     master.lock().unwrap().run().await;
    // });


    // Get disk usage.
    println!("df"); println!("disk free: {:#}", Byte::from_u64(client.df()));
    println!("du"); println!("disk used: {:#}", Byte::from_u64(client.du()));

    // Poll until master has 3 chunkservers free.
    while master.lock().unwrap().get_free_chunkservers(1).len() < 3 {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    // Convert string to bytes.
    let data = "hello world".as_bytes();
    client.append("/test", data, network);

    // Now issue some appends from the client.
    // First client calls master for set of chunkservers to store data.
    // Then client pushes data to chunkservers.
    // Then client calls master to create file and commit with data.
    // Master will call each chunkserver and ask it to commit chunk from LRU.
    // Then it will create the file entry with the chunk locations and commit.

}