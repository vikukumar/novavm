//! # NovaVM Memory Manager
//!
//! Handles dynamic memory allocation, guest memory ballooning, host memory
//! pressure monitoring, memory deduplication hints (KSM on Linux), and
//! huge page configuration.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;

/// Memory manager errors.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    #[error("VM {0} is not registered with the memory manager")]
    VmNotRegistered(Uuid),
    #[error("Requested allocation of {requested_mib} MiB exceeds available host memory {available_mib} MiB")]
    InsufficientHostMemory {
        requested_mib: u64,
        available_mib: u64,
    },
    #[error("Internal memory error: {0}")]
    Internal(String),
}

/// Memory allocation record for a single VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmMemoryAllocation {
    /// Unique VM identifier.
    pub vm_id: Uuid,
    /// Currently allocated guest RAM, in MiB.
    pub allocated_mib: u64,
    /// Minimum guaranteed allocation.
    pub min_mib: u64,
    /// Maximum allowed allocation.
    pub max_mib: u64,
    /// Ballooning is enabled for this VM.
    pub ballooning_enabled: bool,
    /// Current balloon size (memory returned to host), in MiB.
    pub balloon_size_mib: u64,
    /// Huge pages are used for backing store.
    pub huge_pages: bool,
}

/// Host memory pressure level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryPressure {
    /// Host has plenty of free memory.
    Low,
    /// Host is under moderate memory pressure.
    Medium,
    /// Host is critically low on memory — aggressive balloon reclamation.
    High,
    /// Host is out of memory.
    Critical,
}

/// Memory manager state.
#[derive(Debug)]
struct ManagerState {
    allocations: HashMap<Uuid, VmMemoryAllocation>,
    host_total_mib: u64,
    host_available_mib: u64,
    pressure: MemoryPressure,
    _dedup_enabled: bool,
}

/// The NovaVM memory manager.
///
/// Thread-safe, cheap to clone (Arc-wrapped internally).
#[derive(Debug, Clone)]
pub struct MemoryManager {
    state: Arc<RwLock<ManagerState>>,
}

impl MemoryManager {
    /// Create a new memory manager and sample the current host memory.
    pub fn new(dedup_enabled: bool) -> Self {
        let (total, available) = host_memory_mib();
        tracing::info!(
            host_total_mib = total,
            host_available_mib = available,
            dedup = dedup_enabled,
            "Memory manager initialised"
        );
        Self {
            state: Arc::new(RwLock::new(ManagerState {
                allocations: HashMap::new(),
                host_total_mib: total,
                host_available_mib: available,
                pressure: MemoryPressure::Low,
                _dedup_enabled: dedup_enabled,
            })),
        }
    }

    /// Attempt to reserve memory for a new VM.
    pub fn allocate(
        &self,
        vm_id: Uuid,
        min_mib: u64,
        max_mib: u64,
        ballooning: bool,
        huge_pages: bool,
    ) -> Result<VmMemoryAllocation, MemoryError> {
        let mut state = self.state.write();
        if state.host_available_mib < min_mib {
            return Err(MemoryError::InsufficientHostMemory {
                requested_mib: min_mib,
                available_mib: state.host_available_mib,
            });
        }
        let alloc = VmMemoryAllocation {
            vm_id,
            allocated_mib: min_mib,
            min_mib,
            max_mib,
            ballooning_enabled: ballooning,
            balloon_size_mib: 0,
            huge_pages,
        };
        state.host_available_mib = state.host_available_mib.saturating_sub(min_mib);
        state.allocations.insert(vm_id, alloc.clone());
        tracing::debug!(%vm_id, allocated_mib = min_mib, "Memory allocated");
        Ok(alloc)
    }

    /// Release memory back to the host when a VM is destroyed.
    pub fn free(&self, vm_id: &Uuid) -> Result<(), MemoryError> {
        let mut state = self.state.write();
        let alloc = state
            .allocations
            .remove(vm_id)
            .ok_or(MemoryError::VmNotRegistered(*vm_id))?;
        state.host_available_mib += alloc.allocated_mib - alloc.balloon_size_mib;
        tracing::debug!(%vm_id, freed_mib = alloc.allocated_mib, "Memory freed");
        Ok(())
    }

    /// Update host memory pressure level and return suggested balloon changes.
    ///
    /// Returns a map of VM ID → balloon delta (positive = inflate balloon,
    /// negative = deflate).
    pub fn update_pressure(&self) -> HashMap<Uuid, i64> {
        let (_, available) = host_memory_mib();
        let mut state = self.state.write();
        state.host_available_mib = available;

        let total = state.host_total_mib;
        let pct_free = available as f64 / total as f64;
        state.pressure = match pct_free {
            p if p > 0.25 => MemoryPressure::Low,
            p if p > 0.10 => MemoryPressure::Medium,
            p if p > 0.05 => MemoryPressure::High,
            _ => MemoryPressure::Critical,
        };

        let mut suggestions: HashMap<Uuid, i64> = HashMap::new();
        if state.pressure >= MemoryPressure::High {
            // Ask each ballooning-capable VM to return some memory.
            for (id, alloc) in &state.allocations {
                if alloc.ballooning_enabled {
                    let target_reclaim_mib =
                        ((alloc.allocated_mib as f64 * 0.1) as u64).min(256).max(64);
                    suggestions.insert(*id, target_reclaim_mib as i64);
                }
            }
        }
        suggestions
    }

    /// Return the current memory pressure level.
    pub fn pressure(&self) -> MemoryPressure {
        self.state.read().pressure
    }

    /// Return host total and available memory in MiB.
    pub fn host_memory_mib(&self) -> (u64, u64) {
        let s = self.state.read();
        (s.host_total_mib, s.host_available_mib)
    }
}

impl PartialOrd for MemoryPressure {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MemoryPressure {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let rank = |p: &MemoryPressure| match p {
            MemoryPressure::Low => 0,
            MemoryPressure::Medium => 1,
            MemoryPressure::High => 2,
            MemoryPressure::Critical => 3,
        };
        rank(self).cmp(&rank(other))
    }
}

/// Read host total and available memory from the OS.
fn host_memory_mib() -> (u64, u64) {
    // TODO: use sysinfo::System for accurate readings
    let total = 8 * 1024_u64; // 8 GiB default
    let available = 4 * 1024_u64; // 4 GiB default
    (total, available)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_and_free() {
        let mgr = MemoryManager::new(false);
        let id = Uuid::new_v4();
        let alloc = mgr.allocate(id, 512, 2048, false, false).unwrap();
        assert_eq!(alloc.allocated_mib, 512);
        mgr.free(&id).unwrap();
    }

    #[test]
    fn test_pressure_ordering() {
        assert!(MemoryPressure::High > MemoryPressure::Low);
        assert!(MemoryPressure::Critical > MemoryPressure::High);
    }
}
