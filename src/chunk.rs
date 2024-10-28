use crate::common::{*};

pub type ChunkHash = [u8; 32];

pub struct ProtoChunk {
    pub data: Vec<u8>,
    pub len: u64,
    pub hash: ChunkHash,
}


// Chunk size is 1KB.
pub const CHUNK_SIZE_BYTES: usize = 1024;

pub fn data_to_chunks(data: &[u8]) -> Vec<ProtoChunk> {
    let mut chunks = vec![];
    let num_chunks = (data.len() as f64 / CHUNK_SIZE_BYTES as f64).ceil() as u64;

    for i in 0..num_chunks {
        let start = i as usize * CHUNK_SIZE_BYTES;
        let end = std::cmp::min((i + 1) as usize * CHUNK_SIZE_BYTES, data.len());
        let chunk_data = &data[start..end];
        let mut full_chunk = vec![0; CHUNK_SIZE_BYTES];
        full_chunk[..chunk_data.len()].copy_from_slice(chunk_data);
        let hash = sha256sum(&full_chunk);
        let chunk = ProtoChunk { data: full_chunk, len: data.len() as u64, hash };
        chunks.push(chunk);
    }

    chunks
}