//! Snapshot Tauri commands.

use snapshot::SnapshotOrchestrator;
use tauri::State;
use uuid::Uuid;

use api::{ApiError, ApiResult, SnapshotResult};

use crate::state::AppState;

/// Take a snapshot of a VM.
#[tauri::command]
pub async fn take_snapshot(
    vm_id: Uuid,
    name: String,
    description: Option<String>,
    state: State<'_, AppState>,
) -> ApiResult<SnapshotResult> {
    let orchestrator = SnapshotOrchestrator::new(state.engine.clone());
    orchestrator
        .snapshot_vm(vm_id, name, description)
        .await
        .map_err(|e| ApiError::new("SNAPSHOT_ERROR", e.to_string()))
}
