Design
======

I've made some changes to the GFS design. The data flow is much more one way and simpler to understand.

GFS resembles a simple append-only horizontally-scalable storage journal:

 - client pushes data to chunkservers
 - client sends append command to master with datum list and locations
 - master allocates chunk ID's for the datums
 - master asks chunkservers to commit the chunk datums with the ID's
 - master creates the file metadata if it doesn't exist
 - master appends the chunk ID's to the file chunk list

Changes from GFS v1:

 - each chunk has a size and length. in GFS v1, they all had a size which is the same. but in this design, they have a length too. one of the issues of GFS is that if a write straddles a chunk boundary, then we have to lock on the master to allocate the next chunk. in this design, we can append to files as if we are dealing in records.

## Stack.

1. Append-only scalable storage journal