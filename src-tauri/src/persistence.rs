//! Persistence storage engine for VMs and managed disk metadata.

use std::fs;
use std::path::PathBuf;
use api::DiskMetadata;
use engine::VmConfig;

pub struct Persistence {
    base_dir: PathBuf,
}

impl Persistence {
    pub fn new(storage_dir: impl Into<PathBuf>) -> Self {
        let base_dir = storage_dir.into();
        fs::create_dir_all(&base_dir).ok();
        Self { base_dir }
    }

    /// Load all saved VM configurations from disk.
    pub fn load_vms(&self) -> Vec<VmConfig> {
        let file_path = self.base_dir.join("vms.json");
        if !file_path.exists() {
            return Vec::new();
        }

        match fs::read_to_string(&file_path) {
            Ok(json) => match serde_json::from_str::<Vec<VmConfig>>(&json) {
                Ok(vms) => vms,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to parse vms.json");
                    Vec::new()
                }
            },
            Err(e) => {
                tracing::error!(error = %e, "Failed to read vms.json");
                Vec::new()
            }
        }
    }

    /// Save all current VM configurations to disk.
    pub fn save_vms(&self, vms: &[VmConfig]) {
        fs::create_dir_all(&self.base_dir).ok();
        let file_path = self.base_dir.join("vms.json");
        if let Ok(json) = serde_json::to_string_pretty(vms) {
            fs::write(file_path, json).ok();
        }
    }

    /// Load managed disks metadata from disk.
    pub fn load_disks(&self) -> Vec<DiskMetadata> {
        let file_path = self.base_dir.join("disks.json");
        if !file_path.exists() {
            return Vec::new();
        }

        match fs::read_to_string(&file_path) {
            Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    /// Save managed disks metadata to disk.
    pub fn save_disks(&self, disks: &[DiskMetadata]) {
        fs::create_dir_all(&self.base_dir).ok();
        let file_path = self.base_dir.join("disks.json");
        if let Ok(json) = serde_json::to_string_pretty(disks) {
            fs::write(file_path, json).ok();
        }
    }
}
