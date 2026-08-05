//! Storage Tauri commands.

use std::path::PathBuf;
use tauri::State;

use api::{ApiError, ApiResult, DiskFormat, DiskMetadata};

use crate::state::AppState;

/// List all managed disk images.
#[tauri::command]
pub async fn list_disks(state: State<'_, AppState>) -> ApiResult<Vec<DiskMetadata>> {
    Ok(state.disks.read().clone())
}

/// Create a new disk image.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_disk(
    name: String,
    path: String,
    #[allow(non_snake_case)]
    sizeGib: u64,
    thin_provisioned: Option<bool>,
    encrypted: Option<bool>,
    compressed: Option<bool>,
    state: State<'_, AppState>,
) -> ApiResult<DiskMetadata> {
    let size_bytes = sizeGib * 1024 * 1024 * 1024;
    let target_path = if path.trim().is_empty() {
        let storage_dir = state.settings.lock().default_storage_dir.clone();
        std::fs::create_dir_all(&storage_dir).ok();
        PathBuf::from(storage_dir).join(format!("{name}.qcow2"))
    } else {
        PathBuf::from(path)
    };

    let image = storage::DiskImage::create(
        target_path,
        name.clone(),
        size_bytes,
        encrypted.unwrap_or(false),
        compressed.unwrap_or(false),
    )
    .await
    .map_err(|e| ApiError::new("STORAGE_ERROR", e.to_string()))?;

    let mut meta = image.metadata;
    meta.thin_provisioned = thin_provisioned.unwrap_or(true);

    state.disks.write().push(meta.clone());
    state.push_log("INFO", "storage", format!("Disk image '{name}' ({sizeGib} GiB) created successfully"));
    state.sync_disks_to_disk();
    Ok(meta)
}

/// Import an existing disk image or ISO.
#[tauri::command]
pub async fn import_disk(
    path: String,
    name: Option<String>,
    state: State<'_, AppState>,
) -> ApiResult<DiskMetadata> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(ApiError::new("FILE_NOT_FOUND", format!("Disk image file not found at: {path}")));
    }

    let file_name = name.unwrap_or_else(|| {
        p.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "imported-disk".to_owned())
    });

    let ext = p.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let format = match ext.as_str() {
        "iso" => DiskFormat::Raw,
        "vmdk" => DiskFormat::Vmdk,
        "vhd" | "vhdx" => DiskFormat::Vhd,
        "qcow2" => DiskFormat::Qcow2,
        _ => DiskFormat::Raw,
    };

    let size_bytes = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(10 * 1024 * 1024 * 1024);
    let clusters = size_bytes.div_ceil(1024 * 1024);

    let meta = DiskMetadata {
        id: uuid::Uuid::new_v4(),
        name: file_name.clone(),
        path: Some(p),
        virtual_size_bytes: size_bytes,
        cluster_size_bytes: 1024 * 1024,
        allocated_clusters: clusters,
        total_clusters: clusters,
        format,
        encrypted: false,
        compressed: false,
        thin_provisioned: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        parent_snapshot_id: None,
    };

    state.disks.write().push(meta.clone());
    state.push_log("INFO", "storage", format!("Disk/ISO image '{file_name}' imported from '{path}'"));
    state.sync_disks_to_disk();
    Ok(meta)
}

/// Delete a managed disk image.
#[tauri::command]
pub async fn delete_disk(
    id: String,
    delete_file: Option<bool>,
    state: State<'_, AppState>,
) -> ApiResult<()> {
    let parsed_id = uuid::Uuid::parse_str(&id).map_err(|e| ApiError::new("INVALID_ID", e.to_string()))?;
    let mut disks = state.disks.write();
    if let Some(pos) = disks.iter().position(|d| d.id == parsed_id) {
        let removed = disks.remove(pos);
        if delete_file.unwrap_or(false) {
            if let Some(ref disk_path) = removed.path {
                if disk_path.exists() {
                    std::fs::remove_file(disk_path).ok();
                }
            }
        }
        state.push_log("WARN", "storage", format!("Disk image '{name}' deleted", name = removed.name));
        drop(disks);
        state.sync_disks_to_disk();
        Ok(())
    } else {
        Err(ApiError::new("DISK_NOT_FOUND", "Disk image not found in registry"))
    }
}
