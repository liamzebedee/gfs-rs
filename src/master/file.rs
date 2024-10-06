use crate::core::ChunkRef;

struct GFSFile {
    name: String,
    chunks: Vec<ChunkRef>,
    size: u64,
}

struct GFSCreateFileRequest {
    name: String,
    size: u64,
}


struct GFSCreateFileOperation {
    request: GFSCreateFileRequest,
    chunks: Vec<ChunkRef>,
}