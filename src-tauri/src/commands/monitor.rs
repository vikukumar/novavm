//! Monitor Tauri commands — host and VM metrics.

use tauri::State;
use uuid::Uuid;

use api::{ApiError, ApiResult, HostMetrics, VmMetrics};

use crate::state::AppState;

/// Sample and return current host metrics.
#[tauri::command]
pub async fn get_host_metrics(state: State<'_, AppState>) -> ApiResult<HostMetrics> {
    Ok(state.metrics.sample_host())
}

/// Return the latest metrics sample for a specific VM.
#[tauri::command]
pub async fn get_vm_metrics(vm_id: Uuid, state: State<'_, AppState>) -> ApiResult<VmMetrics> {
    state
        .metrics
        .latest_vm(&vm_id)
        .ok_or_else(|| ApiError::new("NO_METRICS", format!("No metrics available for VM {vm_id}")))
}

/// Return host metrics history (ring buffer, up to 300 samples).
#[tauri::command]
pub async fn get_host_metrics_history(state: State<'_, AppState>) -> ApiResult<Vec<HostMetrics>> {
    Ok(state.metrics.host_history())
}

/// Return VM metrics history.
#[tauri::command]
pub async fn get_vm_metrics_history(
    vm_id: Uuid,
    state: State<'_, AppState>,
) -> ApiResult<Vec<VmMetrics>> {
    state
        .metrics
        .vm_history(&vm_id)
        .ok_or_else(|| ApiError::new("NO_METRICS", format!("No history available for VM {vm_id}")))
}

/// Return the real-time application log stream.
#[tauri::command]
pub async fn get_application_logs(state: State<'_, AppState>) -> ApiResult<Vec<api::LogEntry>> {
    Ok(state.logs.read().clone())
}

/// Clear the application log stream.
#[tauri::command]
pub async fn clear_application_logs(state: State<'_, AppState>) -> ApiResult<()> {
    state.logs.write().clear();
    state.push_log("INFO", "novavm_app", "Log history cleared by user");
    Ok(())
}
