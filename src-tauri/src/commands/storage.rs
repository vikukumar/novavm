//! Storage Tauri commands.

use tauri::State;

use api::{ApiError, ApiResult, DiskMetadata};

use crate::state::AppState;

/// List all managed disk images (stub — returns mock data until storage registry is persisted).
#[tauri::command]
pub async fn list_disks(_state: State<'_, AppState>) -> ApiResult<Vec<DiskMetadata>> {
    // TODO: query the on-disk disk registry
    Ok(vec![])
}

/// Create a new disk image.
#[tauri::command]
pub async fn create_disk(
    name: String,
    path: String,
    size_gib: u64,
    encrypted: bool,
    compressed: bool,
    _state: State<'_, AppState>,
) -> ApiResult<DiskMetadata> {
    let image = storage::DiskImage::create(
        std::path::PathBuf::from(path),
        name,
        size_gib * 1024 * 1024 * 1024,
        encrypted,
        compressed,
    )
    .await
    .map_err(|e| ApiError::new("STORAGE_ERROR", e.to_string()))?;
    Ok(image.metadata)
}
