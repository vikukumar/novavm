//! VM configuration types.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Firmware type for a virtual machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirmwareType {
    /// Legacy BIOS firmware.
    Bios,
    /// UEFI firmware (required for Secure Boot).
    Uefi,
}

impl Default for FirmwareType {
    fn default() -> Self {
        Self::Uefi
    }
}

/// CPU configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CpuConfig {
    /// Total number of virtual CPUs presented to the guest.
    pub vcpus: u32,
    /// Number of virtual sockets.
    pub sockets: u32,
    /// Cores per socket.
    pub cores_per_socket: u32,
    /// Threads (hyperthreads) per core.
    pub threads_per_core: u32,
    /// CPU overcommit ratio (1.0 = no overcommit, 2.0 = 2× overcommit).
    pub overcommit_ratio: f32,
}

impl Default for CpuConfig {
    fn default() -> Self {
        Self {
            vcpus: 2,
            sockets: 1,
            cores_per_socket: 2,
            threads_per_core: 1,
            overcommit_ratio: 1.0,
        }
    }
}

/// Memory configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Static memory allocation in MiB.
    pub size_mib: u64,
    /// Minimum dynamic allocation in MiB (when dynamic RAM is enabled).
    pub dynamic_min_mib: u64,
    /// Maximum dynamic allocation in MiB.
    pub dynamic_max_mib: u64,
    /// Enable guest memory ballooning.
    pub ballooning: bool,
    /// Use huge pages for guest memory backing.
    pub huge_pages: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            size_mib: 2048,
            dynamic_min_mib: 512,
            dynamic_max_mib: 4096,
            ballooning: true,
            huge_pages: false,
        }
    }
}

/// Disk bus type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskBus {
    Virtio,
    Scsi,
    Ide,
    Nvme,
}

/// Disk attachment configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiskConfig {
    /// Path to the disk image on the host.
    pub image_path: String,
    /// Bus type.
    pub bus: DiskBus,
    /// Mark as read-only (e.g. for ISO images).
    pub read_only: bool,
    /// Whether this is a boot disk.
    pub boot: bool,
}

/// Network interface type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NicType {
    Virtio,
    E1000,
    Rtl8139,
}

/// Network interface configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NicConfig {
    /// Unique name of the virtual switch to attach to.
    pub switch_name: String,
    /// NIC model.
    pub nic_type: NicType,
    /// Optional static MAC address. If `None`, one is generated.
    pub mac_address: Option<String>,
}

/// Shared folder configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SharedFolderConfig {
    /// Display name inside the guest.
    pub name: String,
    /// Host-side path.
    pub host_path: String,
    /// Whether the guest can write to the folder.
    pub read_only: bool,
    /// Auto-mount in the guest at startup.
    pub auto_mount: bool,
}

/// Top-level VM configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VmConfig {
    /// Human-readable name of the VM.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// CPU configuration.
    pub cpu: CpuConfig,
    /// Memory configuration.
    pub memory: MemoryConfig,
    /// Firmware type.
    pub firmware: FirmwareType,
    /// Enable UEFI Secure Boot (requires UEFI firmware).
    pub secure_boot: bool,
    /// Enable virtual TPM.
    pub vtpm: bool,
    /// Attached disk images.
    pub disks: Vec<DiskConfig>,
    /// Network interfaces.
    pub nics: Vec<NicConfig>,
    /// Shared folders.
    pub shared_folders: Vec<SharedFolderConfig>,
    /// User-defined tags for search and filtering.
    pub tags: Vec<String>,
    /// Optional group name for organising VMs.
    pub group: Option<String>,
}

impl VmConfig {
    /// Create a minimal VM config with sensible defaults.
    pub fn minimal(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            cpu: CpuConfig::default(),
            memory: MemoryConfig::default(),
            firmware: FirmwareType::Uefi,
            secure_boot: false,
            vtpm: false,
            disks: vec![],
            nics: vec![],
            shared_folders: vec![],
            tags: vec![],
            group: None,
        }
    }
}

/// VM template — a named VmConfig used as a starting point for new VMs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmTemplate {
    /// Unique template identifier.
    pub id: Uuid,
    /// Template display name.
    pub name: String,
    /// Description.
    pub description: Option<String>,
    /// The config this template embodies.
    pub config: VmConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_config() {
        let cfg = VmConfig::minimal("my-vm");
        assert_eq!(cfg.name, "my-vm");
        assert_eq!(cfg.cpu.vcpus, 2);
        assert_eq!(cfg.memory.size_mib, 2048);
    }

    #[test]
    fn test_config_roundtrip() {
        let cfg = VmConfig::minimal("roundtrip-test");
        let json = serde_json::to_string(&cfg).unwrap();
        let cfg2: VmConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(cfg, cfg2);
    }
}
