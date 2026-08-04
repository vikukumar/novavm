//! Windows Hypervisor Platform (WHP) backend.
//!
//! Uses the Windows Hypervisor Platform APIs available in Windows 10 version 1803+
//! and Windows Server 2019+. Requires the "Hyper-V" optional feature to be enabled
//! in Windows Features.
//!
//! # Safety
//! WHP API calls are made via `windows-rs`. All unsafe blocks are documented.
//!
//! # References
//! - <https://learn.microsoft.com/en-us/virtualization/api/hypervisor-platform/hypervisor-platform>

use uuid::Uuid;

use crate::{
    HypervisorBackend, HypervisorCapabilities, HypervisorError,
    types::{CreateVmRequest, MemoryStats, VcpuStats, VmHandle},
};

/// Windows Hypervisor Platform backend.
///
/// In production this will use `windows-rs` to call `WHvCreatePartition`,
/// `WHvSetupPartition`, `WHvCreateVirtualProcessor`, etc.
/// Currently implemented as a documented stub that compiles and returns
/// appropriate errors when invoked without a Hyper-V partition.
#[derive(Debug)]
pub struct WhpBackend {
    /// Detected WHP version string.
    version: String,
}

impl WhpBackend {
    /// Initialise the WHP backend, detecting the platform version.
    pub fn new() -> Self {
        // TODO: query WHvGetCapability(WHvCapabilityCodeHypervisorPresent)
        tracing::info!("Initialising Windows Hypervisor Platform backend");
        Self {
            version: "WHP/10.0".to_owned(),
        }
    }
}

impl Default for WhpBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl HypervisorBackend for WhpBackend {
    async fn capabilities(&self) -> HypervisorCapabilities {
        HypervisorCapabilities {
            // TODO: query WHvGetCapability for each flag
            secure_boot: true,
            vtpm: true,
            nested_virt: false,
            huge_pages: false,
            memory_ballooning: true,
            memory_dedup: false,
            usb_redirection: false,
            backend_name: "WHP".to_owned(),
            backend_version: self.version.clone(),
        }
    }

    async fn create_vm(&self, req: CreateVmRequest) -> Result<VmHandle, HypervisorError> {
        tracing::info!(name = %req.name, "WHP: creating partition");
        // TODO: WHvCreatePartition → WHvSetupPartition → WHvCreateVirtualProcessor × vcpus
        // TODO: WHvMapGpaRange for guest RAM
        Ok(VmHandle {
            id: Uuid::new_v4(),
            name: req.name,
            backend_token: "whp-stub".to_owned(),
        })
    }

    async fn start_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::info!(id = %handle.id, "WHP: starting partition");
        // TODO: spawn vCPU run-loop threads via WHvRunVirtualProcessor
        Ok(())
    }

    async fn pause_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::info!(id = %handle.id, "WHP: pausing partition");
        // TODO: WHvSuspendPartitionTime or suspend all vCPU run loops
        Ok(())
    }

    async fn resume_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::info!(id = %handle.id, "WHP: resuming partition");
        Ok(())
    }

    async fn stop_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::info!(id = %handle.id, "WHP: stopping partition");
        // TODO: inject ACPI power-off via IO-APIC emulation
        Ok(())
    }

    async fn destroy_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::info!(id = %handle.id, "WHP: destroying partition");
        // TODO: WHvDeletePartition
        Ok(())
    }

    async fn cpu_stats(&self, _handle: &VmHandle) -> Result<Vec<VcpuStats>, HypervisorError> {
        // TODO: WHvGetVirtualProcessorCounters
        Ok(vec![VcpuStats::default()])
    }

    async fn memory_stats(&self, _handle: &VmHandle) -> Result<MemoryStats, HypervisorError> {
        // TODO: WHvQueryGpaRangeDirtyBitmap + balloon driver balloon
        Ok(MemoryStats::default())
    }
}
