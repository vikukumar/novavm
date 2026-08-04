//! Snapshot management for NovaDisk images.
//!
//! Snapshots form a directed acyclic graph (DAG). Each snapshot records the
//! state of a disk at a point in time. Child images (or subsequent VMs) share
//! clusters with their parents via the copy-on-write (CoW) refcount mechanism.
//!
//! # Snapshot DAG
//!
//! ```text
//!  base-image ──► snap-1 ──► snap-2 (current)
//!                   └──────► snap-1a (branch)
//! ```

use std::{collections::HashMap, path::PathBuf};

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::StorageError;

/// Metadata for a single disk snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Unique snapshot identifier.
    pub id: Uuid,
    /// The disk this snapshot belongs to.
    pub disk_id: Uuid,
    /// Human-readable snapshot name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// When this snapshot was taken.
    pub taken_at: DateTime<Utc>,
    /// Parent snapshot ID (None for the base image).
    pub parent_id: Option<Uuid>,
    /// Number of clusters shared with the parent (via CoW).
    pub shared_clusters: u64,
    /// Number of clusters written to since diverging from parent.
    pub private_clusters: u64,
    /// Whether this snapshot has been exported / backed up.
    pub exported: bool,
}

impl SnapshotMetadata {
    /// Estimate the storage consumed exclusively by this snapshot in bytes.
    pub fn exclusive_size_bytes(&self, cluster_size: u32) -> u64 {
        self.private_clusters * cluster_size as u64
    }
}

/// A snapshot node in the DAG.
#[derive(Debug)]
pub struct Snapshot {
    pub metadata: SnapshotMetadata,
    /// Path to the CoW overlay image for this snapshot.
    pub overlay_path: PathBuf,
    /// Child snapshot IDs.
    pub children: Vec<Uuid>,
}

/// Manager for all snapshots belonging to one disk.
#[derive(Debug, Clone)]
pub struct SnapshotManager {
    inner: Arc<RwLock<SnapshotManagerInner>>,
}

#[derive(Debug)]
struct SnapshotManagerInner {
    disk_id: Uuid,
    snapshots: HashMap<Uuid, Snapshot>,
    /// ID of the snapshot currently being used by the running VM.
    active_snapshot_id: Option<Uuid>,
}

impl SnapshotManager {
    /// Create a new snapshot manager for the given disk.
    pub fn new(disk_id: Uuid) -> Self {
        Self {
            inner: Arc::new(RwLock::new(SnapshotManagerInner {
                disk_id,
                snapshots: HashMap::new(),
                active_snapshot_id: None,
            })),
        }
    }

    /// Take a new snapshot of the current disk state.
    ///
    /// The caller must ensure the VM is paused or stopped before calling this.
    pub fn take_snapshot(
        &self,
        name: String,
        description: Option<String>,
        base_dir: &PathBuf,
    ) -> Result<Uuid, StorageError> {
        let mut inner = self.inner.write();
        let id = Uuid::new_v4();
        let parent_id = inner.active_snapshot_id;
        let overlay_path = base_dir.join(format!("{}.overlay", id));

        let metadata = SnapshotMetadata {
            id,
            disk_id: inner.disk_id,
            name,
            description,
            taken_at: Utc::now(),
            parent_id,
            shared_clusters: 0, // TODO: copy from parent's allocated cluster count
            private_clusters: 0,
            exported: false,
        };

        let snapshot = Snapshot {
            metadata,
            overlay_path,
            children: vec![],
        };

        // Link as child of parent.
        if let Some(parent_id) = parent_id {
            if let Some(parent) = inner.snapshots.get_mut(&parent_id) {
                parent.children.push(id);
            }
        }

        inner.snapshots.insert(id, snapshot);
        inner.active_snapshot_id = Some(id);
        tracing::info!(disk_id = %inner.disk_id, snapshot_id = %id, "Snapshot taken");
        Ok(id)
    }

    /// Revert to a previous snapshot.
    pub fn revert_to(&self, snapshot_id: Uuid) -> Result<(), StorageError> {
        let mut inner = self.inner.write();
        if !inner.snapshots.contains_key(&snapshot_id) {
            return Err(StorageError::SnapshotNotFound(snapshot_id));
        }
        inner.active_snapshot_id = Some(snapshot_id);
        tracing::info!(snapshot_id = %snapshot_id, "Reverted to snapshot");
        Ok(())
    }

    /// Delete a snapshot.
    ///
    /// Merges private clusters into the parent or child as appropriate.
    pub fn delete_snapshot(&self, snapshot_id: Uuid) -> Result<(), StorageError> {
        let mut inner = self.inner.write();
        if !inner.snapshots.contains_key(&snapshot_id) {
            return Err(StorageError::SnapshotNotFound(snapshot_id));
        }
        // TODO: CoW cluster merge
        inner.snapshots.remove(&snapshot_id);
        if inner.active_snapshot_id == Some(snapshot_id) {
            inner.active_snapshot_id = None;
        }
        tracing::info!(snapshot_id = %snapshot_id, "Snapshot deleted");
        Ok(())
    }

    /// List all snapshots in chronological order.
    pub fn list_snapshots(&self) -> Vec<SnapshotMetadata> {
        let inner = self.inner.read();
        let mut metas: Vec<_> = inner
            .snapshots
            .values()
            .map(|s| s.metadata.clone())
            .collect();
        metas.sort_by_key(|m| m.taken_at);
        metas
    }

    /// Return the active snapshot ID.
    pub fn active_snapshot_id(&self) -> Option<Uuid> {
        self.inner.read().active_snapshot_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_take_and_list_snapshots() {
        let dir = tempdir().unwrap();
        let disk_id = Uuid::new_v4();
        let mgr = SnapshotManager::new(disk_id);

        let id1 = mgr
            .take_snapshot("snap-1".to_owned(), None, &dir.path().to_path_buf())
            .unwrap();
        let id2 = mgr
            .take_snapshot("snap-2".to_owned(), None, &dir.path().to_path_buf())
            .unwrap();

        let list = mgr.list_snapshots();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].id, id1);
        assert_eq!(list[1].id, id2);
    }

    #[test]
    fn test_revert_to_snapshot() {
        let dir = tempdir().unwrap();
        let disk_id = Uuid::new_v4();
        let mgr = SnapshotManager::new(disk_id);

        let id1 = mgr
            .take_snapshot("snap-1".to_owned(), None, &dir.path().to_path_buf())
            .unwrap();
        let _id2 = mgr
            .take_snapshot("snap-2".to_owned(), None, &dir.path().to_path_buf())
            .unwrap();

        mgr.revert_to(id1).unwrap();
        assert_eq!(mgr.active_snapshot_id(), Some(id1));
    }
}
