//! # NovaVM Storage Engine
//!
//! Provides the NovaDisk virtual disk container format, snapshot management,
//! AES-256-GCM encryption, and zstd compression for all disk images managed
//! by NovaVM.

pub mod compression;
pub mod disk;
pub mod encryption;
pub mod snapshot;

/// Storage error type.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Disk not found: {0}")]
    DiskNotFound(String),
    #[error("Snapshot not found: {0}")]
    SnapshotNotFound(uuid::Uuid),
    #[error("Disk image is corrupt: {0}")]
    CorruptImage(String),
    #[error("Encryption error: {0}")]
    Encryption(String),
    #[error("Compression error: {0}")]
    Compression(String),
    #[error("Insufficient disk space: need {needed_bytes} bytes, have {available_bytes}")]
    InsufficientSpace { needed_bytes: u64, available_bytes: u64 },
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Internal storage error: {0}")]
    Internal(String),
}

pub use disk::{DiskFormat, DiskImage, DiskMetadata};
pub use encryption::EncryptionKey;
pub use snapshot::{Snapshot, SnapshotManager, SnapshotMetadata};
