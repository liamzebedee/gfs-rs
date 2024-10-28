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

What's the most interesting thing you can build with this? An iPod.

In 10yrs, batteries on mobile devices will be enough that we will be able to serve data very cheaply to anyone anywhere.
So imagine we can just share files:

- source code



Improvements:

 - Reed Solomun codes instead of mirrored replication.



XXXX XXXX XX--

write some data
XXXX

this straddles boundary

XXXX XXXX XX--
            XX XX--

simplify the design
only support appends of a single size!
boom, easy.
mutations though? that's a different matter


writes -> turn into
    mutations
    appends



so long as you can upload as many chunks as possible??

or we're just makign something


















write:

ask master for chunkservers

file 
[] // no chunks

file 
[1] // 1 chunk, partial

file 
[1] // 1 chunk, full



an append should do the following:

if the file has no chunks, simply append
if the file has 1 chunk, not full (empty space), 
  if the append overlaps chunk boundary, we want the append to be atomic in the face of concurrent writers
  
  so simply ping master, sequence the new chunks as pending
