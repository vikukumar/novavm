//! # NovaVM API Facade
//!
//! This crate re-exports all public types from every NovaVM sub-system crate
//! and provides high-level DTOs used by the Tauri command layer.
//!
//! Tauri commands import only this crate rather than individual crates,
//! providing a stable interface boundary.

// Re-export engine types
pub use engine::{Engine, VmConfig, VmRegistry, VmState, VirtualMachine};

// Re-export hypervisor types
pub use hypervisor::{HypervisorBackend, HypervisorCapabilities, HypervisorError};

// Re-export scheduler types
pub use scheduler::{CpuScheduler, VmSchedulingPolicy};

// Re-export memory types
pub use memory::{MemoryManager, MemoryPressure, VmMemoryAllocation};

// Re-export storage types
pub use storage::{DiskFormat, DiskImage, DiskMetadata, SnapshotManager, SnapshotMetadata};

// Re-export network types
pub use network::{NetworkManager, VirtualSwitch, VirtualSwitchMode};

// Re-export snapshot types
pub use snapshot::{SnapshotOrchestrator, SnapshotResult};

// Re-export monitor types
pub use monitor::{HostMetrics, MetricsCollector, VmMetrics};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ─── Request / Response DTOs ─────────────────────────────────────────────────

/// Request to create a new virtual machine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVmRequest {
    pub config: engine::VmConfig,
}

/// Response after creating a VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVmResponse {
    pub vm_id: Uuid,
}

/// A summarised view of a VM suitable for the frontend VM list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmSummary {
    pub id: Uuid,
    pub name: String,
    pub state: VmState,
    pub cpu_vcpus: u32,
    pub memory_mib: u64,
    pub tags: Vec<String>,
    pub group: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Application-level error returned by all Tauri commands.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[error("{message}")]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl ApiError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl From<engine::EngineError> for ApiError {
    fn from(e: engine::EngineError) -> Self {
        Self::new("ENGINE_ERROR", e.to_string())
    }
}

impl From<storage::StorageError> for ApiError {
    fn from(e: storage::StorageError) -> Self {
        Self::new("STORAGE_ERROR", e.to_string())
    }
}

impl From<network::NetworkError> for ApiError {
    fn from(e: network::NetworkError) -> Self {
        Self::new("NETWORK_ERROR", e.to_string())
    }
}

impl From<snapshot::SnapshotError> for ApiError {
    fn from(e: snapshot::SnapshotError) -> Self {
        Self::new("SNAPSHOT_ERROR", e.to_string())
    }
}

/// Helper — convert `Result<T, E>` to `Result<T, ApiError>` where E: Into<ApiError>.
pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_serialization() {
        let err = ApiError::new("TEST_ERROR", "Something went wrong");
        let json = serde_json::to_string(&err).unwrap();
        let parsed: ApiError = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.code, "TEST_ERROR");
    }
}
