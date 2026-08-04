//! Apple Virtualization Framework (AVF) backend.
//!
//! Uses the `Virtualization.framework` (macOS 11+) via Swift/Objective-C FFI
//! or the `vz` Rust crate for hardware-accelerated virtualisation on Apple Silicon
//! and Intel Macs.
//!
//! # References
//! - <https://developer.apple.com/documentation/virtualization>
//! - `vz` crate: <https://github.com/nicholasgasior/vz>

use uuid::Uuid;

use crate::{
    HypervisorBackend, HypervisorCapabilities, HypervisorError,
    types::{CreateVmRequest, MemoryStats, VcpuStats, VmHandle},
};

/// Apple Virtualization Framework backend.
#[derive(Debug)]
pub struct AvfBackend {
    apple_silicon: bool,
}

impl AvfBackend {
    pub fn new() -> Self {
        tracing::info!("Initialising Apple Virtualization Framework backend");
        // TODO: detect Apple Silicon via `sysctl hw.optional.arm64`
        Self { apple_silicon: cfg!(target_arch = "aarch64") }
    }
}

impl Default for AvfBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl HypervisorBackend for AvfBackend {
    async fn capabilities(&self) -> HypervisorCapabilities {
        HypervisorCapabilities {
            secure_boot: self.apple_silicon,
            vtpm: false,
            nested_virt: false,
            huge_pages: false,
            memory_ballooning: true,
            memory_dedup: false,
            usb_redirection: false,
            backend_name: "AVF".to_owned(),
            backend_version: "Virtualization.framework/macOS-14".to_owned(),
        }
    }

    async fn create_vm(&self, req: CreateVmRequest) -> Result<VmHandle, HypervisorError> {
        tracing::info!(name = %req.name, "AVF: creating VZVirtualMachine");
        // TODO: VZVirtualMachineConfiguration → VZVirtualMachine
        Ok(VmHandle {
            id: Uuid::new_v4(),
            name: req.name,
            backend_token: "avf-stub".to_owned(),
        })
    }

    async fn start_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::info!(id = %handle.id, "AVF: starting VZVirtualMachine");
        // TODO: vm.start(completionHandler:)
        Ok(())
    }

    async fn pause_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::info!(id = %handle.id, "AVF: pausing VZVirtualMachine");
        // TODO: vm.pause(completionHandler:)
        Ok(())
    }

    async fn resume_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::info!(id = %handle.id, "AVF: resuming VZVirtualMachine");
        // TODO: vm.resume(completionHandler:)
        Ok(())
    }

    async fn stop_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::info!(id = %handle.id, "AVF: stopping VZVirtualMachine");
        // TODO: vm.requestStop() → vm.stop(completionHandler:)
        Ok(())
    }

    async fn destroy_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::info!(id = %handle.id, "AVF: destroying VZVirtualMachine");
        Ok(())
    }

    async fn cpu_stats(&self, _handle: &VmHandle) -> Result<Vec<VcpuStats>, HypervisorError> {
        Ok(vec![VcpuStats::default()])
    }

    async fn memory_stats(&self, _handle: &VmHandle) -> Result<MemoryStats, HypervisorError> {
        Ok(MemoryStats::default())
    }
}
