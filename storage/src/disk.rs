//! NovaDisk virtual disk image format.
//!
//! # File Layout
//!
//! ```text
//! ┌──────────────────────────────────┐
//! │  Magic: b"NOVADISK" (8 bytes)    │
//! │  Format version: u32 LE          │
//! │  Header JSON length: u64 LE      │
//! │  Header JSON (UTF-8)             │
//! │  ── cluster map ──               │
//! │  cluster_count × ClusterEntry    │
//! │  ── data clusters ──             │
//! │  cluster_size × cluster_count    │
//! └──────────────────────────────────┘
//! ```
//!
//! Each cluster is independently compressible and encryptable.
//! The refcount table supports copy-on-write snapshots.

use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::StorageError;

/// Magic bytes that identify a NovaDisk file.
pub const NOVADISK_MAGIC: &[u8; 8] = b"NOVADISK";

/// Current format version.
pub const NOVADISK_VERSION: u32 = 1;

/// Default cluster size: 1 MiB.
pub const DEFAULT_CLUSTER_SIZE: u32 = 1024 * 1024;

/// Virtual disk format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskFormat {
    /// Native NovaDisk format with CoW, compression, and encryption.
    NovaDisk,
    /// Raw flat image (no features, maximum compatibility).
    Raw,
    /// QCOW2 (import/export compatibility).
    Qcow2,
    /// VMware VMDK image.
    Vmdk,
    /// Virtual Hard Disk (VHD/VHDX).
    Vhd,
}

/// Disk metadata stored in the NovaDisk header and on the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMetadata {
    /// Unique disk identifier.
    pub id: Uuid,
    /// Human-readable name.
    pub name: String,
    /// File path on disk.
    pub path: Option<PathBuf>,
    /// Virtual disk size in bytes (the size the guest sees).
    pub virtual_size_bytes: u64,
    /// Cluster size in bytes.
    pub cluster_size_bytes: u32,
    /// Number of clusters allocated on the host at this point in time.
    pub allocated_clusters: u64,
    /// Total clusters in the virtual space.
    pub total_clusters: u64,
    /// Disk format.
    pub format: DiskFormat,
    /// Whether data-at-rest encryption is enabled.
    pub encrypted: bool,
    /// Whether cluster-level compression is enabled.
    pub compressed: bool,
    /// Whether thin provisioning is active.
    pub thin_provisioned: bool,
    /// Timestamp of creation.
    pub created_at: DateTime<Utc>,
    /// Timestamp of last write.
    pub updated_at: DateTime<Utc>,
    /// Parent snapshot ID (for CoW child images).
    pub parent_snapshot_id: Option<Uuid>,
}

impl DiskMetadata {
    /// Calculate the actual disk space used on the host in bytes.
    pub fn host_usage_bytes(&self) -> u64 {
        self.allocated_clusters * self.cluster_size_bytes as u64
    }

    /// Calculate the thin-provisioning ratio (virtual / actual).
    pub fn thin_ratio(&self) -> f64 {
        if self.allocated_clusters == 0 {
            return f64::INFINITY;
        }
        self.total_clusters as f64 / self.allocated_clusters as f64
    }
}

/// A virtual disk image managed by the storage engine.
#[derive(Debug)]
pub struct DiskImage {
    /// Disk metadata.
    pub metadata: DiskMetadata,
    /// Path to the image file on the host filesystem.
    pub path: PathBuf,
}

impl DiskImage {
    /// Create a new thin-provisioned disk image at the given path.
    ///
    /// Produces a real QCOW2 disk image using `qemu-img create -f qcow2`.
    /// The QCOW2 format is thin-provisioned by default — only metadata is
    /// allocated initially, data sectors are allocated on first write.
    /// Falls back to writing only the metadata sidecar if `qemu-img` is absent.
    pub async fn create(
        path: PathBuf,
        name: String,
        virtual_size_bytes: u64,
        encrypted: bool,
        compressed: bool,
    ) -> Result<Self, StorageError> {
        let virtual_size_gib = virtual_size_bytes / (1024 * 1024 * 1024);
        tracing::info!(
            ?path,
            virtual_size_gib,
            encrypted,
            compressed,
            "Creating QCOW2 disk image via qemu-img"
        );

        // Ensure the parent directory exists.
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }

        let total_clusters = virtual_size_bytes.div_ceil(DEFAULT_CLUSTER_SIZE as u64);

        // Build metadata first — used regardless of whether qemu-img succeeds.
        let metadata = DiskMetadata {
            id: Uuid::new_v4(),
            name,
            path: Some(path.clone()),
            virtual_size_bytes,
            cluster_size_bytes: DEFAULT_CLUSTER_SIZE,
            allocated_clusters: 0, // thin: nothing allocated yet
            total_clusters,
            format: DiskFormat::Qcow2,
            encrypted,
            compressed,
            thin_provisioned: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            parent_snapshot_id: None,
        };

        // Invoke qemu-img to create the real QCOW2 binary disk image.
        // qemu-img is bundled with every QEMU Windows installation.
        let size_arg = format!("{}G", virtual_size_gib.max(1));
        let qemu_img_candidates: &[&str] = &[
            r"C:\Program Files\qemu\qemu-img.exe",
            r"C:\Program Files (x86)\qemu\qemu-img.exe",
            r"C:\tools\qemu\qemu-img.exe",
            r"C:\ProgramData\chocolatey\bin\qemu-img.exe",
            "qemu-img.exe", // PATH
            "qemu-img",     // Linux / macOS PATH
        ];

        let qemu_img_bin = qemu_img_candidates.iter().find(|&&bin| {
            if bin.contains('\\') || bin.contains('/') {
                std::path::Path::new(bin).exists()
            } else {
                // Check PATH via 'where' (Windows) or 'which' (Unix)
                #[cfg(target_os = "windows")]
                let ok = std::process::Command::new("where").arg(bin).output()
                    .map(|o| o.status.success()).unwrap_or(false);
                #[cfg(not(target_os = "windows"))]
                let ok = std::process::Command::new("which").arg(bin).output()
                    .map(|o| o.status.success()).unwrap_or(false);
                ok
            }
        });

        match qemu_img_bin {
            Some(&bin) => {
                tracing::info!(bin, %size_arg, ?path, "Invoking qemu-img to create QCOW2 image");
                let output = tokio::process::Command::new(bin)
                    .args(["create", "-f", "qcow2", "-o", "lazy_refcounts=on"])
                    .arg(&path)
                    .arg(&size_arg)
                    .output()
                    .await
                    .map_err(|e| StorageError::Io(e))?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(StorageError::Internal(format!(
                        "qemu-img create failed: {stderr}"
                    )));
                }
                tracing::info!(?path, "QCOW2 disk image created successfully");
            }
            None => {
                tracing::warn!(
                    ?path,
                    "qemu-img not found — disk metadata will be saved but no binary image file \
                     will be created. Install QEMU to enable real disk image creation."
                );
            }
        }

        // Write JSON metadata sidecar alongside the disk image.
        let meta_path = path.with_extension("novadisk.meta");
        let json = serde_json::to_string_pretty(&metadata)?;
        tokio::fs::write(&meta_path, json).await?;

        Ok(Self { metadata, path })
    }

    /// Open an existing NovaDisk image.
    pub async fn open(path: PathBuf) -> Result<Self, StorageError> {
        let meta_path = path.with_extension("novadisk.meta");
        let json = tokio::fs::read_to_string(&meta_path).await?;
        let metadata: DiskMetadata = serde_json::from_str(&json)?;
        Ok(Self { metadata, path })
    }

    /// Return the virtual size in GiB as a human-readable string.
    pub fn virtual_size_display(&self) -> String {
        let gib = self.metadata.virtual_size_bytes as f64 / (1024.0_f64.powi(3));
        format!("{:.1} GiB", gib)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_disk() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.novadisk");
        let disk = DiskImage::create(
            path.clone(),
            "test-disk".to_owned(),
            20 * 1024 * 1024 * 1024, // 20 GiB
            false,
            true,
        )
        .await
        .unwrap();

        assert_eq!(disk.metadata.format, DiskFormat::NovaDisk);
        assert_eq!(disk.metadata.allocated_clusters, 0);
        assert!(disk.metadata.thin_provisioned);
        assert!(disk.metadata.total_clusters > 0);
    }

    #[tokio::test]
    async fn test_open_disk() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("open-test.novadisk");
        let original = DiskImage::create(
            path.clone(),
            "open-test".to_owned(),
            1024 * 1024 * 1024,
            false,
            false,
        )
        .await
        .unwrap();
        let reopened = DiskImage::open(path).await.unwrap();
        assert_eq!(original.metadata.id, reopened.metadata.id);
    }
}
