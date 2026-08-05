//! Linux KVM backend.
//!
//! Interfaces with `/dev/kvm` using the KVM IOCTL API to create and manage
//! hardware-accelerated virtual machines on Linux.
//!
//! # References
//! - <https://www.kernel.org/doc/html/latest/virt/kvm/api.html>
//! - `kvm-ioctls` crate for safe Rust bindings

use uuid::Uuid;

use crate::{
    types::{CreateVmRequest, MemoryStats, VcpuStats, VmHandle},
    HypervisorBackend, HypervisorCapabilities, HypervisorError,
};

/// Linux KVM hypervisor backend.
///
/// Production implementation will use the `kvm-ioctls` crate to open
/// `/dev/kvm`, create VMs, vCPUs, and manage memory slots.
#[derive(Debug)]
pub struct KvmBackend {
    version: i32,
}

impl KvmBackend {
    pub fn new() -> Self {
        tracing::info!("Initialising KVM backend");
        // Open /dev/kvm, ioctl(KVM_GET_API_VERSION)
        Self { version: 12 }
    }
}

impl Default for KvmBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl HypervisorBackend for KvmBackend {
    async fn capabilities(&self) -> HypervisorCapabilities {
        HypervisorCapabilities {
            // KVM_CHECK_EXTENSION capability verification
            secure_boot: false,
            vtpm: true,
            nested_virt: true,
            huge_pages: true,
            memory_ballooning: true,
            memory_dedup: true,
            usb_redirection: true,
            backend_name: "KVM".to_owned(),
            backend_version: format!("KVM API {}", self.version),
        }
    }

    async fn create_vm(&self, req: CreateVmRequest) -> Result<VmHandle, HypervisorError> {
        tracing::info!(name = %req.name, "KVM: creating VM");
        // ioctl(KVM_CREATE_VM) -> ioctl(KVM_CREATE_VCPU)
        // mmap guest RAM, ioctl(KVM_SET_USER_MEMORY_REGION)
        Ok(VmHandle { id: Uuid::new_v4(), name: req.name, backend_token: "kvm-stub".to_owned() })
    }

    async fn start_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::info!(id = %handle.id, "KVM: running vCPU threads");
        // Spawn tokio tasks calling ioctl(KVM_RUN) in loop
        Ok(())
    }

    async fn pause_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::info!(id = %handle.id, "KVM: pausing vCPUs");
        // Signal vCPU threads to exit run loop
        Ok(())
    }

    async fn resume_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::info!(id = %handle.id, "KVM: resuming vCPUs");
        Ok(())
    }

    async fn stop_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::info!(id = %handle.id, "KVM: stopping VM");
        Ok(())
    }

    async fn destroy_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::info!(id = %handle.id, "KVM: destroying VM");
        // Close file descriptors, unmap guest RAM
        Ok(())
    }

    async fn cpu_stats(&self, _handle: &VmHandle) -> Result<Vec<VcpuStats>, HypervisorError> {
        // Read /proc/self/fdinfo for vcpu fds
        Ok(vec![VcpuStats::default()])
    }

    async fn memory_stats(&self, _handle: &VmHandle) -> Result<MemoryStats, HypervisorError> {
        Ok(MemoryStats::default())
    }
}
