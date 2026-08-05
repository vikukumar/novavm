//! Settings Tauri commands.

use tauri::State;

use api::ApiResult;
use hypervisor::nova_engine::{detect_nova_engine, NovaEngineCapabilities};

use crate::state::{AppSettings, AppState};

/// Get current application settings.
#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> ApiResult<AppSettings> {
    Ok(state.settings.lock().clone())
}

/// Update application settings.
#[tauri::command]
pub async fn update_settings(settings: AppSettings, state: State<'_, AppState>) -> ApiResult<()> {
    *state.settings.lock() = settings;
    // Settings updated in shared application state.
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

/// Check whether QEMU is installed and return its installation status.
#[tauri::command]
pub async fn get_qemu_status() -> serde_json::Value {
    let candidates: &[&str] = &[
        r"C:\Program Files\qemu\qemu-system-x86_64.exe",
        r"C:\Program Files (x86)\qemu\qemu-system-x86_64.exe",
        r"C:\tools\qemu\qemu-system-x86_64.exe",
        r"C:\ProgramData\chocolatey\bin\qemu-system-x86_64.exe",
    ];

    // Try PATH first
    let from_path = std::process::Command::new("where")
        .arg("qemu-system-x86_64.exe")
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok().map(|s| s.lines().next().unwrap_or("").trim().to_owned())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty());

    if let Some(path) = from_path {
        return serde_json::json!({ "installed": true, "path": path });
    }

    for &candidate in candidates {
        if std::path::Path::new(candidate).exists() {
            return serde_json::json!({ "installed": true, "path": candidate });
        }
    }

    serde_json::json!({
        "installed": false,
        "path": null,
        "install_url": "https://www.qemu.org/download/#windows",
        "message": "QEMU is not installed. Virtual machines cannot start until QEMU is installed. Download from https://www.qemu.org/download/#windows"
    })
}

/// Return full NovaVM virtualization engine capabilities.
/// This is the primary source of truth for the Dashboard virtualization card.
#[tauri::command]
pub async fn get_virtualization_info() -> NovaEngineCapabilities {
    tokio::task::spawn_blocking(detect_nova_engine)
        .await
        .unwrap_or_else(|_| detect_nova_engine())
}
