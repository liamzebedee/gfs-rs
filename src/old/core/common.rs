/// Reference to a chunk.
pub type ChunkRef = u64;

/// Address of a chunkserver.
pub struct ChunkserverAddress {
    ip: String,
    port: u16,
}

/// Information about a chunkserver.
pub struct ChunkserverInfo {
    lastSeen: u64,
    ip: String,
    port: u16,
    chunks: Vec<ChunkRef>,
}