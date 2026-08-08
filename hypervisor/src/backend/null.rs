//! Null hypervisor backend â does nothing, always succeeds.
//!
//! Used in unit tests and on platforms where no native backend is available.

use uuid::Uuid;

use crate::{
    types::{CreateVmRequest, MemoryStats, VcpuStats, VmHandle},
    HypervisorBackend, HypervisorCapabilities, HypervisorError,
};

/// A no-op hypervisor backend that satisfies the trait contract without
/// touching any OS API. Safe on all platforms.
#[derive(Debug, Clone, Copy)]
pub struct NullBackend;

#[async_trait::async_trait]
impl HypervisorBackend for NullBackend {
    async fn capabilities(&self) -> HypervisorCapabilities {
        HypervisorCapabilities {
            backend_name: "null".to_owned(),
            backend_version: "0.0.0".to_owned(),
            ..Default::default()
        }
    }

    async fn create_vm(&self, req: CreateVmRequest) -> Result<VmHandle, HypervisorError> {
        tracing::debug!(name = %req.name, vcpus = req.vcpus, memory_mib = req.memory_mib, "NullBackend::create_vm");
        Ok(VmHandle { id: req.id.unwrap_or_else(Uuid::new_v4), name: req.name, backend_token: "null".to_owned() })
    }

    async fn start_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::debug!(id = %handle.id, "NullBackend::start_vm");
        Ok(())
    }

    async fn pause_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::debug!(id = %handle.id, "NullBackend::pause_vm");
        Ok(())
    }

    async fn resume_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::debug!(id = %handle.id, "NullBackend::resume_vm");
        Ok(())
    }

    async fn stop_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::debug!(id = %handle.id, "NullBackend::stop_vm");
        Ok(())
    }

    async fn destroy_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        tracing::debug!(id = %handle.id, "NullBackend::destroy_vm");
        Ok(())
    }

    async fn cpu_stats(&self, _handle: &VmHandle) -> Result<Vec<VcpuStats>, HypervisorError> {
        Ok(vec![VcpuStats::default()])
    }

    async fn memory_stats(&self, _handle: &VmHandle) -> Result<MemoryStats, HypervisorError> {
        Ok(MemoryStats::default())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

