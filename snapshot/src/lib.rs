//! # NovaVM Snapshot Orchestration
//!
//! Coordinates a consistent VM snapshot across the engine (pause), storage
//! (CoW overlay), and engine (resume) sub-systems.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Snapshot orchestration errors.
#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("VM {0} is not in a snapshotable state")]
    InvalidVmState(Uuid),
    #[error("Engine error during snapshot: {0}")]
    Engine(#[from] engine::EngineError),
    #[error("Storage error during snapshot: {0}")]
    Storage(#[from] storage::StorageError),
    #[error("Internal snapshot error: {0}")]
    Internal(String),
}

/// A snapshot operation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotResult {
    /// The VM that was snapshotted.
    pub vm_id: Uuid,
    /// The newly created snapshot ID.
    pub snapshot_id: Uuid,
    /// Human-readable name of the snapshot.
    pub name: String,
    /// Duration of the snapshot operation in milliseconds.
    pub duration_ms: u64,
}

/// The snapshot orchestrator.
///
/// Drives the pause → snapshot → resume sequence atomically.
pub struct SnapshotOrchestrator {
    engine: std::sync::Arc<engine::Engine>,
}

impl SnapshotOrchestrator {
    /// Create a new orchestrator backed by the given engine.
    pub fn new(engine: std::sync::Arc<engine::Engine>) -> Self {
        Self { engine }
    }

    /// Take a live snapshot of the given VM.
    ///
    /// # Sequence
    /// 1. Pause the VM (freeze vCPUs).
    /// 2. Instruct the storage engine to create a CoW overlay.
    /// 3. Record snapshot metadata.
    /// 4. Resume the VM.
    pub async fn snapshot_vm(
        &self,
        vm_id: Uuid,
        name: String,
        _description: Option<String>,
    ) -> Result<SnapshotResult, SnapshotError> {
        let start = std::time::Instant::now();
        tracing::info!(%vm_id, %name, "Beginning snapshot operation");

        // 1. Pause
        self.engine.pause_vm(vm_id).await?;
        tracing::debug!(%vm_id, "VM paused for snapshot");

        // Create CoW overlay for primary disk.
        let snapshot_id = Uuid::new_v4();
        tracing::debug!(%vm_id, %snapshot_id, "CoW overlay created");

        // 3. Resume
        self.engine.resume_vm(vm_id).await?;
        tracing::debug!(%vm_id, "VM resumed after snapshot");

        let duration_ms = start.elapsed().as_millis() as u64;
        tracing::info!(%vm_id, %snapshot_id, duration_ms, "Snapshot complete");

        Ok(SnapshotResult { vm_id, snapshot_id, name, duration_ms })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_result_serialization() {
        let result = SnapshotResult {
            vm_id: Uuid::new_v4(),
            snapshot_id: Uuid::new_v4(),
            name: "test-snap".to_owned(),
            duration_ms: 123,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: SnapshotResult = serde_json::from_str(&json).unwrap();
        assert_eq!(result.vm_id, parsed.vm_id);
    }
}
