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
}

impl Engine {
    /// Create and initialise a new engine instance.
    pub fn new() -> Self {
        tracing::info!("Initialising NovaVM engine");
        Self { registry: VmRegistry::new() }
    }

    /// Expose the VM registry for external use (e.g. Tauri commands).
    pub fn registry(&self) -> &VmRegistry {
        &self.registry
    }

    /// Create a new VM from the given configuration, register it, and return its UUID.
    #[instrument(skip(self))]
    pub async fn create_vm(&self, config: VmConfig) -> Result<Uuid, EngineError> {
        tracing::info!(name = %config.name, "Creating VM");
        let vm = VirtualMachine::new(config).await?;
        let id = self.registry.insert(vm);
        tracing::info!(%id, "VM created");
        Ok(id)
    }

    /// Start a VM that is currently stopped.
    #[instrument(skip(self))]
    pub async fn start_vm(&self, id: Uuid) -> Result<(), EngineError> {
        let handle = self.registry.get(&id).ok_or(EngineError::VmNotFound(id))?;
        let mut vm = handle.write().await;
        vm.start().await
    }

    /// Pause a running VM.
    #[instrument(skip(self))]
    pub async fn pause_vm(&self, id: Uuid) -> Result<(), EngineError> {
        let handle = self.registry.get(&id).ok_or(EngineError::VmNotFound(id))?;
        let mut vm = handle.write().await;
        vm.pause().await
    }

    /// Resume a paused VM.
    #[instrument(skip(self))]
    pub async fn resume_vm(&self, id: Uuid) -> Result<(), EngineError> {
        let handle = self.registry.get(&id).ok_or(EngineError::VmNotFound(id))?;
        let mut vm = handle.write().await;
        vm.resume().await
    }

    /// Stop (graceful shutdown) a running VM.
    #[instrument(skip(self))]
    pub async fn stop_vm(&self, id: Uuid) -> Result<(), EngineError> {
        let handle = self.registry.get(&id).ok_or(EngineError::VmNotFound(id))?;
        let mut vm = handle.write().await;
        vm.stop().await
    }

    /// Hard-reset a VM (equivalent to pressing the physical reset button).
    #[instrument(skip(self))]
    pub async fn reset_vm(&self, id: Uuid) -> Result<(), EngineError> {
        let handle = self.registry.get(&id).ok_or(EngineError::VmNotFound(id))?;
        let mut vm = handle.write().await;
        vm.reset().await
    }

    /// Destroy a VM — stop it if running and remove from registry.
    #[instrument(skip(self))]
    pub async fn destroy_vm(&self, id: Uuid) -> Result<(), EngineError> {
        let handle = self.registry.get(&id).ok_or(EngineError::VmNotFound(id))?;
        {
            let mut vm = handle.write().await;
            vm.destroy().await?;
        }
        self.registry.remove(&id);
        tracing::info!(%id, "VM destroyed and removed from registry");
        Ok(())
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
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
