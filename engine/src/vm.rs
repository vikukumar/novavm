//! Virtual machine state machine and lifecycle management.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use uuid::Uuid;

use crate::{config::VmConfig, error::EngineError};

/// All observable states a virtual machine can be in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmState {
    /// The VM is not running and no resources are allocated.
    Stopped,
    /// The VM is in the process of starting.
    Starting,
    /// The VM is running and accepting workloads.
    Running,
    /// The VM is paused — vCPUs are frozen but memory is retained.
    Paused,
    /// The VM encountered a fatal error and is no longer running.
    Crashed,
    /// The VM is saving its state to disk.
    Saving,
    /// The VM is being restored from a saved state.
    Restoring,
    /// The VM is being cloned (source side).
    Cloning,
    /// The VM is in the process of being destroyed.
    Destroying,
}

impl std::fmt::Display for VmState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VmState::Stopped => write!(f, "stopped"),
            VmState::Starting => write!(f, "starting"),
            VmState::Running => write!(f, "running"),
            VmState::Paused => write!(f, "paused"),
            VmState::Crashed => write!(f, "crashed"),
            VmState::Saving => write!(f, "saving"),
            VmState::Restoring => write!(f, "restoring"),
            VmState::Cloning => write!(f, "cloning"),
            VmState::Destroying => write!(f, "destroying"),
        }
    }
}

/// Runtime statistics for a virtual machine.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VmStats {
    /// CPU usage across all vCPUs, as a percentage (0–100).
    pub cpu_percent: f64,
    /// Guest RAM in use, in MiB.
    pub memory_used_mib: u64,
    /// Disk read throughput in bytes per second.
    pub disk_read_bps: u64,
    /// Disk write throughput in bytes per second.
    pub disk_write_bps: u64,
    /// Network receive throughput in bytes per second.
    pub net_rx_bps: u64,
    /// Network transmit throughput in bytes per second.
    pub net_tx_bps: u64,
}

/// A virtual machine managed by the NovaVM engine.
#[derive(Debug, Serialize, Deserialize)]
pub struct VirtualMachine {
    /// Stable unique identifier assigned at creation time.
    id: Uuid,
    /// User-provided configuration.
    config: VmConfig,
    /// Current lifecycle state.
    state: VmState,
    /// Timestamp when the VM was created.
    created_at: DateTime<Utc>,
    /// Timestamp of the most recent state change.
    updated_at: DateTime<Utc>,
    /// Latest statistics (populated while running).
    stats: VmStats,
}

impl VirtualMachine {
    /// Construct a new VM from configuration.
    ///
    /// Validates the configuration and initialises storage / network resources.
    pub async fn new(config: VmConfig) -> Result<Self, EngineError> {
        Self::validate_config(&config)?;
        let now = Utc::now();
        Ok(Self {
            id: Uuid::new_v4(),
            config,
            state: VmState::Stopped,
            created_at: now,
            updated_at: now,
            stats: VmStats::default(),
        })
    }

    // ─── Accessors ────────────────────────────────────────────────────────────

    /// Return the VM's stable unique identifier.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Return a reference to the VM's configuration.
    pub fn config(&self) -> &VmConfig {
        &self.config
    }

    /// Return a mutable reference to the VM's configuration (for live editing).
    pub fn config_mut(&mut self) -> &mut VmConfig {
        &mut self.config
    }

    /// Return the current state.
    pub fn state(&self) -> &VmState {
        &self.state
    }

    /// Return the creation timestamp.
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Return the last-updated timestamp.
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Return the latest runtime statistics.
    pub fn stats(&self) -> &VmStats {
        &self.stats
    }

    // ─── Lifecycle ────────────────────────────────────────────────────────────

    /// Start the VM. Only valid from `Stopped` or `Crashed`.
    #[instrument(skip(self), fields(id = %self.id))]
    pub async fn start(&mut self) -> Result<(), EngineError> {
        match &self.state {
            VmState::Stopped | VmState::Crashed => {}
            s => {
                return Err(EngineError::InvalidStateTransition {
                    action: "start",
                    current_state: s.to_string(),
                })
            }
        }
        tracing::info!("Starting VM {}", self.id);
        self.transition(VmState::Starting);
        // Invoke hypervisor backend to create and start the guest.
        self.transition(VmState::Running);
        Ok(())
    }

    /// Pause the VM. Only valid from `Running`.
    #[instrument(skip(self), fields(id = %self.id))]
    pub async fn pause(&mut self) -> Result<(), EngineError> {
        self.require_state(VmState::Running, "pause")?;
        tracing::info!("Pausing VM {}", self.id);
        // Invoke hypervisor backend to pause vCPUs.
        self.transition(VmState::Paused);
        Ok(())
    }

    /// Resume a paused VM. Only valid from `Paused`.
    #[instrument(skip(self), fields(id = %self.id))]
    pub async fn resume(&mut self) -> Result<(), EngineError> {
        self.require_state(VmState::Paused, "resume")?;
        tracing::info!("Resuming VM {}", self.id);
        // Invoke hypervisor backend to resume vCPUs.
        self.transition(VmState::Running);
        Ok(())
    }

    /// Gracefully stop the VM. Valid from `Running` or `Paused`.
    #[instrument(skip(self), fields(id = %self.id))]
    pub async fn stop(&mut self) -> Result<(), EngineError> {
        match &self.state {
            VmState::Running | VmState::Paused => {}
            s => {
                return Err(EngineError::InvalidStateTransition {
                    action: "stop",
                    current_state: s.to_string(),
                })
            }
        }
        tracing::info!("Stopping VM {}", self.id);
        // Send ACPI shutdown signal via hypervisor backend.
        self.transition(VmState::Stopped);
        Ok(())
    }

    /// Hard-reset the VM. Only valid from `Running`.
    #[instrument(skip(self), fields(id = %self.id))]
    pub async fn reset(&mut self) -> Result<(), EngineError> {
        self.require_state(VmState::Running, "reset")?;
        tracing::info!("Resetting VM {}", self.id);
        // Invoke hypervisor backend to reset the virtual machine.
        self.transition(VmState::Starting);
        self.transition(VmState::Running);
        Ok(())
    }

    /// Destroy the VM — release all hypervisor resources.
    ///
    /// The caller is responsible for removing the VM from the registry afterwards.
    #[instrument(skip(self), fields(id = %self.id))]
    pub async fn destroy(&mut self) -> Result<(), EngineError> {
        tracing::info!("Destroying VM {}", self.id);
        self.transition(VmState::Destroying);
        // Invoke hypervisor backend to destroy the guest.
        self.transition(VmState::Stopped);
        Ok(())
    }

    /// Update the statistics snapshot (called by the monitor sub-system).
    pub fn update_stats(&mut self, stats: VmStats) {
        self.stats = stats;
        self.updated_at = Utc::now();
    }

    // ─── Private helpers ──────────────────────────────────────────────────────

    fn transition(&mut self, next: VmState) {
        tracing::debug!(id = %self.id, from = %self.state, to = %next, "State transition");
        self.state = next;
        self.updated_at = Utc::now();
    }

    /// Force the VM into Stopped state without validating from-state.
    /// Used for rollback after a failed hypervisor start.
    pub fn force_stopped(&mut self) {
        self.transition(VmState::Stopped);
    }

    /// Force the VM into Running state without validating from-state.
    /// Used when we re-create a hypervisor handle for a restored VM.
    pub fn force_running(&mut self) {
        self.transition(VmState::Running);
    }

    fn require_state(&self, required: VmState, action: &'static str) -> Result<(), EngineError> {
        if self.state != required {
            Err(EngineError::InvalidStateTransition {
                action,
                current_state: self.state.to_string(),
            })
        } else {
            Ok(())
        }
    }

    fn validate_config(cfg: &VmConfig) -> Result<(), EngineError> {
        if cfg.name.trim().is_empty() {
            return Err(EngineError::Config("VM name must not be empty".to_owned()));
        }
        if cfg.cpu.vcpus == 0 {
            return Err(EngineError::Config("VM must have at least one vCPU".to_owned()));
        }
        if cfg.memory.size_mib == 0 {
            return Err(EngineError::Config("VM memory must be > 0 MiB".to_owned()));
        }
        if cfg.secure_boot && cfg.firmware != crate::config::FirmwareType::Uefi {
            return Err(EngineError::Config("Secure Boot requires UEFI firmware".to_owned()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CpuConfig, FirmwareType, MemoryConfig};

    fn make_config() -> VmConfig {
        VmConfig {
            name: "unit-test-vm".to_owned(),
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

    #[tokio::test]
    async fn test_lifecycle_happy_path() {
        let mut vm = VirtualMachine::new(make_config()).await.unwrap();
        assert_eq!(*vm.state(), VmState::Stopped);
        vm.start().await.unwrap();
        assert_eq!(*vm.state(), VmState::Running);
        vm.pause().await.unwrap();
        assert_eq!(*vm.state(), VmState::Paused);
        vm.resume().await.unwrap();
        assert_eq!(*vm.state(), VmState::Running);
        vm.stop().await.unwrap();
        assert_eq!(*vm.state(), VmState::Stopped);
    }

    #[tokio::test]
    async fn test_invalid_transition() {
        let mut vm = VirtualMachine::new(make_config()).await.unwrap();
        // Cannot pause a stopped VM.
        assert!(vm.pause().await.is_err());
    }

    #[tokio::test]
    async fn test_validation_empty_name() {
        let mut cfg = make_config();
        cfg.name = "   ".to_owned();
        assert!(VirtualMachine::new(cfg).await.is_err());
    }

    #[tokio::test]
    async fn test_validation_secure_boot_no_uefi() {
        let mut cfg = make_config();
        cfg.secure_boot = true;
        cfg.firmware = FirmwareType::Bios;
        assert!(VirtualMachine::new(cfg).await.is_err());
    }
}
