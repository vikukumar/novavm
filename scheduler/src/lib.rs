//! # NovaVM CPU Scheduler
//!
//! Manages vCPU-to-pCPU affinity, overcommit ratios, and priority-based
//! scheduling across all running virtual machines.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tracing::instrument;
use uuid::Uuid;

/// Error type for scheduler operations.
#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    #[error("VM {0} is not registered with the scheduler")]
    VmNotRegistered(Uuid),
    #[error("Invalid overcommit ratio {0}: must be ≥ 1.0")]
    InvalidOvercommitRatio(f32),
    #[error("Internal scheduler error: {0}")]
    Internal(String),
}

/// Scheduling policy for a single VM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VmSchedulingPolicy {
    /// Number of virtual CPUs.
    pub vcpus: u32,
    /// Priority weight relative to other VMs (1–100, higher = more CPU time).
    pub priority: u8,
    /// Optional affinity mask: which host logical processors this VM may use.
    /// Empty means "use any".
    pub cpu_affinity: Vec<u32>,
    /// Per-VM overcommit override (if None, the global ratio is used).
    pub overcommit_ratio: Option<f32>,
}

impl Default for VmSchedulingPolicy {
    fn default() -> Self {
        Self {
            vcpus: 2,
            priority: 50,
            cpu_affinity: vec![],
            overcommit_ratio: None,
        }
    }
}

/// Host CPU topology information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuTopology {
    /// Total logical CPUs visible to the hypervisor host.
    pub logical_cpus: u32,
    /// Physical cores.
    pub physical_cores: u32,
    /// Physical sockets.
    pub sockets: u32,
    /// Hyperthreading available.
    pub hyperthreading: bool,
}

/// Scheduler state.
#[derive(Debug, Default)]
struct SchedulerState {
    /// Per-VM scheduling policies.
    policies: HashMap<Uuid, VmSchedulingPolicy>,
    /// Global CPU overcommit ratio.
    global_overcommit_ratio: f32,
    /// Detected host CPU topology.
    topology: CpuTopology,
}

/// The NovaVM CPU scheduler.
///
/// Thread-safe and cheap to clone — internally `Arc`-wrapped.
#[derive(Debug, Clone)]
pub struct CpuScheduler {
    state: Arc<RwLock<SchedulerState>>,
}

impl CpuScheduler {
    /// Create a new scheduler with the given global overcommit ratio.
    pub fn new(global_overcommit_ratio: f32) -> Result<Self, SchedulerError> {
        if global_overcommit_ratio < 1.0 {
            return Err(SchedulerError::InvalidOvercommitRatio(
                global_overcommit_ratio,
            ));
        }
        let topology = Self::detect_topology();
        tracing::info!(
            logical_cpus = topology.logical_cpus,
            physical_cores = topology.physical_cores,
            overcommit = global_overcommit_ratio,
            "CPU scheduler initialised"
        );
        Ok(Self {
            state: Arc::new(RwLock::new(SchedulerState {
                policies: HashMap::new(),
                global_overcommit_ratio,
                topology,
            })),
        })
    }

    /// Register a VM with its scheduling policy.
    #[instrument(skip(self))]
    pub fn register_vm(&self, vm_id: Uuid, policy: VmSchedulingPolicy) {
        let mut state = self.state.write();
        state.policies.insert(vm_id, policy);
        tracing::debug!(%vm_id, "VM registered with scheduler");
    }

    /// Deregister a VM when it is destroyed.
    pub fn deregister_vm(&self, vm_id: &Uuid) {
        self.state.write().policies.remove(vm_id);
        tracing::debug!(%vm_id, "VM deregistered from scheduler");
    }

    /// Update the scheduling policy for a running VM.
    pub fn update_policy(
        &self,
        vm_id: Uuid,
        policy: VmSchedulingPolicy,
    ) -> Result<(), SchedulerError> {
        let mut state = self.state.write();
        if !state.policies.contains_key(&vm_id) {
            return Err(SchedulerError::VmNotRegistered(vm_id));
        }
        state.policies.insert(vm_id, policy);
        Ok(())
    }

    /// Return the effective vCPU budget for a VM, taking overcommit into account.
    pub fn effective_vcpu_budget(&self, vm_id: &Uuid) -> Result<f32, SchedulerError> {
        let state = self.state.read();
        let policy = state
            .policies
            .get(vm_id)
            .ok_or(SchedulerError::VmNotRegistered(*vm_id))?;
        let ratio = policy
            .overcommit_ratio
            .unwrap_or(state.global_overcommit_ratio);
        Ok(policy.vcpus as f32 / ratio)
    }

    /// Return how many vCPUs are currently scheduled across all VMs.
    pub fn total_scheduled_vcpus(&self) -> u32 {
        self.state
            .read()
            .policies
            .values()
            .map(|p| p.vcpus)
            .sum()
    }

    /// Return the detected host CPU topology.
    pub fn topology(&self) -> CpuTopology {
        self.state.read().topology.clone()
    }

    /// Detect host CPU topology using sysinfo / OS APIs.
    fn detect_topology() -> CpuTopology {
        // TODO: use `sysinfo` or raw OS APIs for accurate topology
        CpuTopology {
            logical_cpus: num_cpus(),
            physical_cores: (num_cpus() / 2).max(1),
            sockets: 1,
            hyperthreading: num_cpus() > 1,
        }
    }
}

/// Returns the number of logical CPUs available to the process.
fn num_cpus() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let sched = CpuScheduler::new(1.5).unwrap();
        assert!(sched.total_scheduled_vcpus() == 0);
    }

    #[test]
    fn test_invalid_overcommit() {
        assert!(CpuScheduler::new(0.5).is_err());
    }

    #[test]
    fn test_register_and_budget() {
        let sched = CpuScheduler::new(2.0).unwrap();
        let id = Uuid::new_v4();
        sched.register_vm(
            id,
            VmSchedulingPolicy {
                vcpus: 4,
                ..Default::default()
            },
        );
        // With 4 vCPUs and 2× global overcommit, effective budget = 2.0
        let budget = sched.effective_vcpu_budget(&id).unwrap();
        assert!((budget - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_deregister() {
        let sched = CpuScheduler::new(1.0).unwrap();
        let id = Uuid::new_v4();
        sched.register_vm(id, VmSchedulingPolicy::default());
        sched.deregister_vm(&id);
        assert!(sched.effective_vcpu_budget(&id).is_err());
    }
}
