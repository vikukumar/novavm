//! # NovaVM Performance Monitor
//!
//! Collects real-time host and per-VM metrics using sysinfo. Maintains a
//! ring-buffer history for sparkline charts in the frontend.

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Monitor errors.
#[derive(Debug, thiserror::Error)]
pub enum MonitorError {
    #[error("VM {0} not registered for monitoring")]
    VmNotMonitored(Uuid),
    #[error("Internal monitor error: {0}")]
    Internal(String),
}

/// A single snapshot of host metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HostMetrics {
    /// Host CPU usage across all logical CPUs, 0–100%.
    pub cpu_percent: f64,
    /// Host total RAM in MiB.
    pub memory_total_mib: u64,
    /// Host used RAM in MiB.
    pub memory_used_mib: u64,
    /// Host swap total in MiB.
    pub swap_total_mib: u64,
    /// Host swap used in MiB.
    pub swap_used_mib: u64,
    /// Per-CPU usages in percent.
    pub per_cpu_percent: Vec<f64>,
    /// Total host disk read in bytes since last sample.
    pub disk_read_bytes: u64,
    /// Total host disk write in bytes since last sample.
    pub disk_write_bytes: u64,
    /// Total host network received bytes since last sample.
    pub net_rx_bytes: u64,
    /// Total host network transmitted bytes since last sample.
    pub net_tx_bytes: u64,
    /// UNIX timestamp (seconds) of this sample.
    pub timestamp: u64,
}

/// A single snapshot of VM-level metrics.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VmMetrics {
    /// VM unique identifier.
    pub vm_id: Uuid,
    /// CPU usage 0–100% across all vCPUs.
    pub cpu_percent: f64,
    /// Guest memory used in MiB.
    pub memory_used_mib: u64,
    /// Disk read bytes since last sample.
    pub disk_read_bytes: u64,
    /// Disk write bytes since last sample.
    pub disk_write_bytes: u64,
    /// Network received bytes since last sample.
    pub net_rx_bytes: u64,
    /// Network transmitted bytes since last sample.
    pub net_tx_bytes: u64,
    /// UNIX timestamp (seconds) of this sample.
    pub timestamp: u64,
}

/// Maximum number of historical samples retained per entity.
const RING_BUFFER_SIZE: usize = 300; // 5 minutes at 1 Hz

/// The metrics collector.
///
/// Samples host and VM metrics on a configurable interval and retains a
/// ring-buffer of history for charting.
#[derive(Clone)]
pub struct MetricsCollector {
    host_history: Arc<RwLock<VecDeque<HostMetrics>>>,
    vm_history: Arc<DashMap<Uuid, VecDeque<VmMetrics>>>,
}

impl MetricsCollector {
    /// Create a new metrics collector.
    pub fn new() -> Self {
        Self {
            host_history: Arc::new(RwLock::new(VecDeque::with_capacity(RING_BUFFER_SIZE))),
            vm_history: Arc::new(DashMap::new()),
        }
    }

    /// Register a VM for metric collection.
    pub fn register_vm(&self, vm_id: Uuid) {
        self.vm_history.entry(vm_id).or_insert_with(|| VecDeque::with_capacity(RING_BUFFER_SIZE));
    }

    /// Deregister a VM.
    pub fn deregister_vm(&self, vm_id: &Uuid) {
        self.vm_history.remove(vm_id);
    }

    /// Collect one sample of host metrics.
    pub fn sample_host(&self) -> HostMetrics {
        let metrics = collect_host_metrics();
        let mut history = self.host_history.write();
        if history.len() >= RING_BUFFER_SIZE {
            history.pop_front();
        }
        history.push_back(metrics.clone());
        metrics
    }

    /// Inject a VM metrics sample (typically supplied by the hypervisor backend).
    pub fn record_vm_metrics(&self, metrics: VmMetrics) {
        let mut entry = self
            .vm_history
            .entry(metrics.vm_id)
            .or_insert_with(|| VecDeque::with_capacity(RING_BUFFER_SIZE));
        if entry.len() >= RING_BUFFER_SIZE {
            entry.pop_front();
        }
        entry.push_back(metrics);
    }

    /// Return the most recent host metrics sample.
    pub fn latest_host(&self) -> Option<HostMetrics> {
        self.host_history.read().back().cloned()
    }

    /// Return the most recent VM metrics sample.
    pub fn latest_vm(&self, vm_id: &Uuid) -> Option<VmMetrics> {
        self.vm_history.get(vm_id).and_then(|h| h.back().cloned())
    }

    /// Return the full host metrics history.
    pub fn host_history(&self) -> Vec<HostMetrics> {
        self.host_history.read().iter().cloned().collect()
    }

    /// Return the full VM metrics history.
    pub fn vm_history(&self, vm_id: &Uuid) -> Option<Vec<VmMetrics>> {
        self.vm_history.get(vm_id).map(|h| h.iter().cloned().collect())
    }

    /// Run background metrics collection loop sampling host metrics every `interval`.
    pub async fn run_background_collection(self, interval: Duration) {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            self.sample_host();
            tracing::trace!("Host metrics sampled");
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Collect current host metrics from sysinfo.
fn collect_host_metrics() -> HostMetrics {
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_all();

    // sysinfo 0.31+ returns bytes — convert to MiB
    let total_mib = sys.total_memory() / (1024 * 1024);
    let used_mib = sys.used_memory() / (1024 * 1024);
    let swap_total_mib = sys.total_swap() / (1024 * 1024);
    let swap_used_mib = sys.used_swap() / (1024 * 1024);
    let per_cpu: Vec<f64> = sys.cpus().iter().map(|c| c.cpu_usage() as f64).collect();
    let cpu_avg =
        if per_cpu.is_empty() { 0.0 } else { per_cpu.iter().sum::<f64>() / per_cpu.len() as f64 };

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    HostMetrics {
        cpu_percent: cpu_avg,
        memory_total_mib: total_mib,
        memory_used_mib: used_mib,
        swap_total_mib,
        swap_used_mib,
        per_cpu_percent: per_cpu,
        disk_read_bytes: 0,
        disk_write_bytes: 0,
        net_rx_bytes: 0,
        net_tx_bytes: 0,
        timestamp: ts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_host() {
        let collector = MetricsCollector::new();
        let metrics = collector.sample_host();
        assert!(metrics.memory_total_mib > 0);
    }

    #[test]
    fn test_ring_buffer_limit() {
        let collector = MetricsCollector::new();
        for _ in 0..RING_BUFFER_SIZE + 10 {
            collector.sample_host();
        }
        assert_eq!(collector.host_history().len(), RING_BUFFER_SIZE);
    }

    #[test]
    fn test_vm_metrics_record() {
        let collector = MetricsCollector::new();
        let vm_id = Uuid::new_v4();
        collector.register_vm(vm_id);
        collector.record_vm_metrics(VmMetrics { vm_id, cpu_percent: 42.5, ..Default::default() });
        let latest = collector.latest_vm(&vm_id).unwrap();
        assert!((latest.cpu_percent - 42.5).abs() < 0.001);
    }
}
