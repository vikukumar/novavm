//! zstd cluster-level compression.
//!
//! Each cluster is compressed independently using `zstd`. This allows
//! random access to any cluster without decompressing the whole image.

use crate::StorageError;

/// Default zstd compression level (1–22). 3 is the recommended balance.
pub const DEFAULT_COMPRESSION_LEVEL: i32 = 3;

/// Compress a cluster payload using zstd.
pub fn compress_cluster(data: &[u8], level: i32) -> Result<Vec<u8>, StorageError> {
    zstd::bulk::compress(data, level).map_err(|e| StorageError::Compression(e.to_string()))
}

/// Decompress a cluster payload using zstd.
pub fn decompress_cluster(
    compressed: &[u8],
    max_output_size: usize,
) -> Result<Vec<u8>, StorageError> {
    zstd::bulk::decompress(compressed, max_output_size)
        .map_err(|e| StorageError::Compression(e.to_string()))
}

/// Compute the compression ratio of a block.
pub fn compression_ratio(original_size: usize, compressed_size: usize) -> f64 {
    if compressed_size == 0 {
        return f64::INFINITY;
    }
    original_size as f64 / compressed_size as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress() {
        let data = b"Hello, NovaDisk compression! ".repeat(100);
        let compressed = compress_cluster(&data, DEFAULT_COMPRESSION_LEVEL).unwrap();
        assert!(compressed.len() < data.len(), "Should compress well");
        let decompressed = decompress_cluster(&compressed, data.len() * 2).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn test_compression_ratio() {
        let ratio = compression_ratio(1000, 200);
        assert!((ratio - 5.0).abs() < f64::EPSILON);
    }
}
