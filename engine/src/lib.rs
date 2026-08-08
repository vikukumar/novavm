//! # NovaVM Engine
//!
//! Core VM lifecycle engine. Owns the authoritative registry of every virtual machine,
//! drives state transitions, and coordinates sub-systems (hypervisor, scheduler, memory,
//! storage, network).

pub mod config;
pub mod error;
pub mod vm;

use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::RwLock;
use tracing::instrument;
use uuid::Uuid;

use hypervisor::{detect_backend, types::CreateVmRequest, HypervisorBackend};

pub use config::VmConfig;
pub use error::EngineError;
pub use vm::{VirtualMachine, VmState};

/// Central registry of all virtual machines managed by this engine instance.
///
/// `VmRegistry` is cheap to clone — it wraps an `Arc`.
#[derive(Clone, Debug, Default)]
pub struct VmRegistry {
    inner: Arc<RegistryInner>,
}

#[derive(Debug, Default)]
struct RegistryInner {
    vms: DashMap<Uuid, Arc<RwLock<VirtualMachine>>>,
}

impl VmRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a newly created VM and return its ID.
    pub fn insert(&self, vm: VirtualMachine) -> Uuid {
        let id = vm.id();
        self.inner.vms.insert(id, Arc::new(RwLock::new(vm)));
        id
    }

    /// Look up a VM by ID.
    pub fn get(&self, id: &Uuid) -> Option<Arc<RwLock<VirtualMachine>>> {
        self.inner.vms.get(id).map(|r| Arc::clone(r.value()))
    }

    /// Remove a VM from the registry (after destroy).
    pub fn remove(&self, id: &Uuid) -> Option<Arc<RwLock<VirtualMachine>>> {
        self.inner.vms.remove(id).map(|(_, v)| v)
    }

    /// Return all VM IDs in the registry.
    pub fn ids(&self) -> Vec<Uuid> {
        self.inner.vms.iter().map(|r| *r.key()).collect()
    }

    /// Return the number of registered VMs.
    pub fn len(&self) -> usize {
        self.inner.vms.len()
    }

    /// Return true if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.vms.is_empty()
    }
}

/// Top-level engine handle. Holds references to all sub-systems.
pub struct Engine {
    registry: VmRegistry,
    /// The active hypervisor backend (QEMU process, WHP, KVM, etc.)
    hypervisor: Arc<dyn HypervisorBackend>,
    /// Hypervisor-level VM handles, keyed by engine-level VM UUID.
    handles: DashMap<Uuid, hypervisor::types::VmHandle>,
}

impl Engine {
    /// Create and initialise a new engine instance.
    pub fn new() -> Self {
        let hypervisor = detect_backend();
        tracing::info!("Initialising NovaVM engine");
        Self {
            registry: VmRegistry::new(),
            hypervisor,
            handles: DashMap::new(),
        }
    }

    /// Expose the VM registry for external use (e.g. Tauri commands).
    pub fn registry(&self) -> &VmRegistry {
        &self.registry
    }

    /// Expose the active hypervisor backend.
    /// Can be downcast to a concrete type for backend-specific features.
    pub fn hypervisor(&self) -> Arc<dyn HypervisorBackend> {
        Arc::clone(&self.hypervisor)
    }

    /// Create a new VM from the given configuration, register it, and return its UUID.
    #[instrument(skip(self))]
    pub async fn create_vm(&self, config: VmConfig) -> Result<Uuid, EngineError> {
        tracing::info!(name = %config.name, "Creating VM");

        // Extract disk and ISO paths from the config's disk list.
        // The first writable disk is the primary hard disk; the first read-only disk is the ISO.
        let disk_path = config.disks.iter()
            .find(|d| !d.read_only)
            .map(|d| d.image_path.clone());
        let iso_path = config.disks.iter()
            .find(|d| d.read_only)
            .map(|d| d.image_path.clone());

        let vm = VirtualMachine::new(config).await?;
        let id = vm.id();

        // Build the hypervisor-level create request with synchronized VM UUID
        let hyp_req = CreateVmRequest {
            id: Some(id),
            name: vm.config().name.clone(),
            vcpus: vm.config().cpu.vcpus,
            memory_mib: vm.config().memory.size_mib,
            firmware: match vm.config().firmware {
                config::FirmwareType::Uefi => hypervisor::types::FirmwareType::Uefi,
                config::FirmwareType::Bios => hypervisor::types::FirmwareType::Bios,
            },
            secure_boot: vm.config().secure_boot,
            vtpm: vm.config().vtpm,
            disk_path,
            iso_path,
        };

        // Create hypervisor-level handle (allocates backend resources / QEMU params)
        let hyp_handle = self.hypervisor.create_vm(hyp_req).await.map_err(|e| {
            EngineError::Hypervisor(e.to_string())
        })?;

        self.registry.insert(vm);

        // Store the hypervisor handle so start/stop can use it
        self.handles.insert(id, hyp_handle);

        tracing::info!(%id, "VM created");
        Ok(id)
    }

    /// Start a VM that is currently stopped.
    #[instrument(skip(self))]
    pub async fn start_vm(&self, id: Uuid) -> Result<(), EngineError> {
        let handle = self.registry.get(&id).ok_or(EngineError::VmNotFound(id))?;
        let mut vm = handle.write().await;

        // Get the hypervisor handle (may not exist if VM was restored from JSON)
        let hyp_handle_opt = self.handles.get(&id).map(|r| r.clone());

        vm.start().await?;

        // Launch the real QEMU process
        if let Some(hyp_handle) = hyp_handle_opt {
            if let Err(e) = self.hypervisor.start_vm(&hyp_handle).await {
                tracing::error!(%id, error = %e, "Hypervisor failed to start VM — state rolled back");
                // Roll back state
                vm.force_stopped();
                return Err(EngineError::Hypervisor(e.to_string()));
            }
        } else {
            tracing::warn!(%id, "No hypervisor handle found for VM — creating fresh one");
            // VM may have been restored from JSON without a live handle.
            // Re-create a hypervisor handle from the VM config.
            let config = vm.config().clone();
            drop(vm); // release write lock before creating handle
            let disk_path = config.disks.iter()
                .find(|d| !d.read_only)
                .map(|d| d.image_path.clone());
            let iso_path = config.disks.iter()
                .find(|d| d.read_only)
                .map(|d| d.image_path.clone());
            let hyp_req = CreateVmRequest {
                id: Some(id),
                name: config.name.clone(),
                vcpus: config.cpu.vcpus,
                memory_mib: config.memory.size_mib,
                firmware: match config.firmware {
                    config::FirmwareType::Uefi => hypervisor::types::FirmwareType::Uefi,
                    config::FirmwareType::Bios => hypervisor::types::FirmwareType::Bios,
                },
                secure_boot: config.secure_boot,
                vtpm: config.vtpm,
                disk_path,
                iso_path,
            };
            let hyp_handle = self.hypervisor.create_vm(hyp_req).await.map_err(|e| {
                EngineError::Hypervisor(e.to_string())
            })?;
            if let Err(e) = self.hypervisor.start_vm(&hyp_handle).await {
                return Err(EngineError::Hypervisor(e.to_string()));
            }
            self.handles.insert(id, hyp_handle);
            // Re-acquire and set running state
            let handle2 = self.registry.get(&id).ok_or(EngineError::VmNotFound(id))?;
            let mut vm2 = handle2.write().await;
            vm2.force_running();
            return Ok(());
        }

        Ok(())
    }

    /// Pause a running VM.
    #[instrument(skip(self))]
    pub async fn pause_vm(&self, id: Uuid) -> Result<(), EngineError> {
        let handle = self.registry.get(&id).ok_or(EngineError::VmNotFound(id))?;
        let mut vm = handle.write().await;
        vm.pause().await?;
        if let Some(hyp_handle) = self.handles.get(&id) {
            let _ = self.hypervisor.pause_vm(&hyp_handle).await;
        }
        Ok(())
    }

    /// Resume a paused VM.
    #[instrument(skip(self))]
    pub async fn resume_vm(&self, id: Uuid) -> Result<(), EngineError> {
        let handle = self.registry.get(&id).ok_or(EngineError::VmNotFound(id))?;
        let mut vm = handle.write().await;
        vm.resume().await?;
        if let Some(hyp_handle) = self.handles.get(&id) {
            let _ = self.hypervisor.resume_vm(&hyp_handle).await;
        }
        Ok(())
    }

    /// Stop (graceful shutdown) a running VM.
    #[instrument(skip(self))]
    pub async fn stop_vm(&self, id: Uuid) -> Result<(), EngineError> {
        let handle = self.registry.get(&id).ok_or(EngineError::VmNotFound(id))?;
        let mut vm = handle.write().await;
        vm.stop().await?;
        if let Some(hyp_handle) = self.handles.get(&id) {
            let _ = self.hypervisor.stop_vm(&hyp_handle).await;
        }
        Ok(())
    }

    /// Hard-reset a VM (equivalent to pressing the physical reset button).
    #[instrument(skip(self))]
    pub async fn reset_vm(&self, id: Uuid) -> Result<(), EngineError> {
        let handle = self.registry.get(&id).ok_or(EngineError::VmNotFound(id))?;
        let mut vm = handle.write().await;
        vm.reset().await?;
        // For QEMU: stop + start the process
        if let Some(hyp_handle) = self.handles.get(&id) {
            let _ = self.hypervisor.stop_vm(&hyp_handle).await;
            let _ = self.hypervisor.start_vm(&hyp_handle).await;
        }
        Ok(())
    }

    /// Destroy a VM — stop it if running and remove from registry.
    #[instrument(skip(self))]
    pub async fn destroy_vm(&self, id: Uuid) -> Result<(), EngineError> {
        let handle = self.registry.get(&id).ok_or(EngineError::VmNotFound(id))?;
        {
            let mut vm = handle.write().await;
            vm.destroy().await?;
            if let Some(hyp_handle) = self.handles.get(&id) {
                let _ = self.hypervisor.destroy_vm(&hyp_handle).await;
            }
        }
        self.handles.remove(&id);
        self.registry.remove(&id);
        tracing::info!(%id, "VM destroyed and removed from registry");
        Ok(())
    }

    /// Query live performance metrics for a running VM.
    pub async fn sample_vm_metrics(&self, id: Uuid) -> Option<monitor::VmMetrics> {
        let hyp_handle = self.handles.get(&id)?.clone();
        let cpu_stats = self.hypervisor.cpu_stats(&hyp_handle).await.ok()?;
        let mem_stats = self.hypervisor.memory_stats(&hyp_handle).await.ok()?;

        let total_cpu = if cpu_stats.is_empty() {
            0.0
        } else {
            cpu_stats
                .iter()
                .map(|s| s.guest_percent + s.hypervisor_percent)
                .sum::<f64>()
                / cpu_stats.len() as f64
        };

        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Some(monitor::VmMetrics {
            vm_id: id,
            cpu_percent: total_cpu,
            memory_used_mib: mem_stats.used_mib,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            net_rx_bytes: 0,
            net_tx_bytes: 0,
            timestamp: ts,
        })
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine")
            .field("vm_count", &self.registry.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CpuConfig, FirmwareType, MemoryConfig};

    fn test_config(name: &str) -> VmConfig {
        VmConfig {
            name: name.to_owned(),
            description: Some("Test VM".to_owned()),
            cpu: CpuConfig {
                vcpus: 2,
                sockets: 1,
                cores_per_socket: 2,
                threads_per_core: 1,
                overcommit_ratio: 1.0,
            },
            memory: MemoryConfig {
                size_mib: 512,
                dynamic_min_mib: 256,
                dynamic_max_mib: 1024,
                ballooning: false,
                huge_pages: false,
            },
            firmware: FirmwareType::Bios,
            secure_boot: false,
            vtpm: false,
            disks: vec![],
            nics: vec![],
            shared_folders: vec![],
            tags: vec![],
            group: None,
        }
    }

    #[tokio::test]
    async fn test_create_vm() {
        let engine = Engine::new();
        let id = engine.create_vm(test_config("test-vm-1")).await.unwrap();
        assert_eq!(engine.registry().len(), 1);
        assert!(engine.registry().get(&id).is_some());
    }

    #[tokio::test]
    async fn test_destroy_vm() {
        let engine = Engine::new();
        let id = engine.create_vm(test_config("test-vm-2")).await.unwrap();
        engine.destroy_vm(id).await.unwrap();
        assert!(engine.registry().is_empty());
    }

    #[tokio::test]
    async fn test_vm_not_found() {
        let engine = Engine::new();
        let fake_id = Uuid::new_v4();
        let result = engine.start_vm(fake_id).await;
        assert!(matches!(result, Err(EngineError::VmNotFound(_))));
    }
}

