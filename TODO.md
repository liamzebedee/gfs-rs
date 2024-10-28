How to institute the master write logic?

Client asks master for chunk ID allocations
Client asks master for chunkservers
Client writes chunks
Client contacts master with chunk locations
Master verifies each location and then updates file


Who would use that? 
What about writing full chunks vs. partial chunks? 
I don't know. The write size - does it really matter? We're going over the network. Surely you can write smaller sizes.
The only issue is that the master is keeping reference to the size. So typically you have a full block size.


What is the problem?
We need a scalable data storage system for blockchains.
Let's just design a file storage system originally.
Important thing is that the UX needs to allow for uploading files like S3.
It needs to be scalable. But ideally it's a single network to begin with. Storage scales out. And maybe it's a connectable metadata system.
Kinda like IPFS but if there was a file index on top LOL.

I think it's easier if there is decoupling between the master and the chunkservers
even though the master technically is just maintaining an index.
I think having chunkservers write full blocks is important though
it helps reason about the system

I don't know
make something really bad and then simplify

simply put-

simplify chunkserver selection algo
then push data to each chunkserver in parallel
then ask master to create allocate chunk ids





importing ethereum state history into goliath

something like tinychain
writing blocks to goliath and then being done with that




I think writes look like this:


File
    Write
        Data (N bytes) -> ceil(N//K) -> [CHUNK; K]

then you just sequence at master for the list of chunk ids.


Simply delete the chunk straddling. Make the design simpler. 
What if a chunk no longer fills the entire length? Idk. Store the chunk length? Or make it application-defined? 
I think just store the chunk lengths.

Each chunk then: 
id: u64
len: u32 [varint]

































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