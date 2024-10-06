use crate::core::*;

pub struct ChunkserverProcess {
    chunkset: Vec<ChunkRef>,   
}

impl ChunkserverProcess {
    pub fn new() -> ChunkserverProcess {
        ChunkserverProcess {
            chunkset: Vec::new(),
        }
    }
}