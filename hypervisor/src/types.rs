//! Common types shared across all hypervisor backends.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Firmware selection mirrored from engine::config for use in hypervisor requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FirmwareType {
    Bios,
    Uefi,
}

/// Parameters for creating a new VM at the hypervisor level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVmRequest {
    /// Optional explicitly requested VM UUID. If None, backend generates a new Uuid.
    pub id: Option<Uuid>,
    /// Human-readable VM name (used for logging / OS-level object names).
    pub name: String,
    /// Number of virtual CPUs.
    pub vcpus: u32,
    /// Amount of guest RAM in MiB.
    pub memory_mib: u64,
    /// Firmware type.
    pub firmware: FirmwareType,
    /// Enable Secure Boot (requires UEFI).
    pub secure_boot: bool,
    /// Enable virtual TPM.
    pub vtpm: bool,
    /// Path to the primary virtual hard disk image on the host.
    /// For QEMU: QCOW2 or any QEMU-supported format.
    /// For VirtualBox: VDI, VMDK, or VHD.
    pub disk_path: Option<String>,
    /// Path to an installer ISO or optical disc image to boot from.
    /// This is attached as a CD-ROM / DVD drive with highest boot priority.
    pub iso_path: Option<String>,
}

/// An opaque handle to a hypervisor-level VM object.
///
/// Backends store all platform-specific data inside this handle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmHandle {
    /// Stable identifier assigned by the backend at creation time.
    pub id: Uuid,
    /// The name supplied in the create request.
    pub name: String,
    /// Opaque backend-specific token (serialised as a JSON string for portability).
    pub backend_token: String,
}

/// Per-vCPU statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VcpuStats {
    /// vCPU index (0-based).
    pub index: u32,
    /// Percentage of time the vCPU was in guest mode (0–100).
    pub guest_percent: f64,
    /// Percentage of time the vCPU was in hypervisor mode (0–100).
    pub hypervisor_percent: f64,
    /// Percentage idle time (0–100).
    pub idle_percent: f64,
}

/// Guest memory statistics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Total guest RAM visible to the OS, in MiB.
    pub total_mib: u64,
    /// RAM currently in use by the guest OS, in MiB.
    pub used_mib: u64,
    /// RAM available (free + reclaimable), in MiB.
    pub available_mib: u64,
    /// Amount of memory reclaimed by the balloon driver, in MiB.
    pub balloon_size_mib: u64,
}
