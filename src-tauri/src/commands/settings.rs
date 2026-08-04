//! Settings Tauri commands.

use tauri::State;

use api::ApiResult;

use crate::state::{AppSettings, AppState};

/// Get current application settings.
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> ApiResult<AppSettings> {
    Ok(state.settings.lock().clone())
}

/// Update application settings.
#[tauri::command]
pub async fn update_settings(
    settings: AppSettings,
    state: State<'_, AppState>,
) -> ApiResult<()> {
    *state.settings.lock() = settings;
    // TODO: persist to config file
    Ok(())
}

/// Return the NovaVM application version.
#[tauri::command]
pub async fn get_app_version() -> ApiResult<String> {
    Ok(env!("CARGO_PKG_VERSION").to_owned())
}

/// Return hypervisor capabilities for the current platform.
#[tauri::command]
pub async fn get_hypervisor_info() -> ApiResult<serde_json::Value> {
    let backend = hypervisor::detect_backend();
    let caps = backend.capabilities().await;
    Ok(serde_json::json!({
        "backend_name": caps.backend_name,
        "backend_version": caps.backend_version,
        "secure_boot": caps.secure_boot,
        "vtpm": caps.vtpm,
        "nested_virt": caps.nested_virt,
        "huge_pages": caps.huge_pages,
        "memory_ballooning": caps.memory_ballooning,
        "memory_dedup": caps.memory_dedup,
        "usb_redirection": caps.usb_redirection,
    }))
}
