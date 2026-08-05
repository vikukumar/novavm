//! Network Tauri commands.

use tauri::State;

use api::{ApiError, ApiResult, VirtualSwitch, VirtualSwitchMode};

use crate::state::AppState;

/// List all virtual switches.
#[tauri::command]
pub async fn list_switches(state: State<'_, AppState>) -> ApiResult<Vec<VirtualSwitch>> {
    Ok(state.network.list_switches())
}

/// Create a virtual switch.
#[tauri::command]
pub async fn create_switch(
    name: String,
    mode: VirtualSwitchMode,
    state: State<'_, AppState>,
) -> ApiResult<String> {
    let id = state
        .network
        .create_switch(name, mode)
        .map_err(|e| ApiError::new("NETWORK_ERROR", e.to_string()))?;
    Ok(id.to_string())
}

/// Delete a virtual switch.
#[tauri::command]
pub async fn delete_switch(name: String, state: State<'_, AppState>) -> ApiResult<()> {
    state.network.delete_switch(&name).map_err(|e| ApiError::new("NETWORK_ERROR", e.to_string()))
}
