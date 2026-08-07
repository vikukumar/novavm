//! # NovaVM Hypervisor Abstraction
//!
//! This crate defines the [`HypervisorBackend`] async trait and provides
//! platform-specific implementations:
//!
//! | Platform | Backend | Feature gate |
//! |----------|---------|-------------|
//! | Windows  | Windows Hypervisor Platform (WHP / Hyper-V) | `cfg(target_os = "windows")` |
//! | Linux    | KVM (`/dev/kvm`) | `cfg(target_os = "linux")` |
//! | macOS    | Apple Virtualization Framework | `cfg(target_os = "macos")` |
//!
//! A [`NullBackend`] is always available for testing and CI on any OS.

pub mod backend;
pub mod device;
pub mod nova_engine;
pub mod types;

use std::sync::Arc;

pub use types::*;

/// All errors that can originate from a hypervisor backend.
#[derive(Debug, thiserror::Error)]
pub enum HypervisorError {
    #[error("Hypervisor not available on this platform: {0}")]
    NotAvailable(String),
    #[error("Failed to create virtual machine: {0}")]
    CreateFailed(String),
    #[error("Failed to start virtual machine: {0}")]
    StartFailed(String),
    #[error("Failed to pause virtual machine: {0}")]
    PauseFailed(String),
    #[error("Failed to resume virtual machine: {0}")]
    ResumeFailed(String),
    #[error("Failed to stop virtual machine: {0}")]
    StopFailed(String),
    #[error("Failed to destroy virtual machine: {0}")]
    DestroyFailed(String),
    #[error("Memory operation failed: {0}")]
    MemoryError(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Internal hypervisor error: {0}")]
    Internal(String),
}

/// Capability flags reported by a hypervisor backend.
#[derive(Debug, Clone, Default)]
pub struct HypervisorCapabilities {
    /// Backend can enforce UEFI Secure Boot.
    pub secure_boot: bool,
    /// Backend supports a virtual TPM 2.0 device.
    pub vtpm: bool,
    /// Backend supports nested virtualisation.
    pub nested_virt: bool,
    /// Backend supports huge-page-backed guest memory.
    pub huge_pages: bool,
    /// Backend supports memory ballooning.
    pub memory_ballooning: bool,
    /// Backend supports memory deduplication (KSM).
    pub memory_dedup: bool,
    /// Backend supports USB device redirection.
    pub usb_redirection: bool,
    /// Name of the detected backend.
    pub backend_name: String,
    /// Backend version string.
    pub backend_version: String,
}

/// The core async trait every hypervisor backend must implement.
///
/// Implementors are expected to be `Send + Sync` so they can be used across
/// tokio tasks and held behind `Arc<dyn HypervisorBackend>`.
#[async_trait::async_trait]
pub trait HypervisorBackend: Send + Sync + std::fmt::Debug {
    /// Return the capability set of this backend.
    async fn capabilities(&self) -> HypervisorCapabilities;

    /// Allocate and initialise hypervisor-level resources for a new VM.
    async fn create_vm(&self, req: CreateVmRequest) -> Result<VmHandle, HypervisorError>;

    /// Start (boot) a previously created VM.
    async fn start_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError>;

    /// Freeze all vCPUs (guest state is preserved in RAM).
    async fn pause_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError>;

    /// Unfreeze all vCPUs.
    async fn resume_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError>;

    /// Send an ACPI power-down request and wait for guest shutdown.
    async fn stop_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError>;

    /// Immediately terminate and release all hypervisor resources for the VM.
    async fn destroy_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError>;

    /// Query live vCPU usage statistics for all vCPUs.
    async fn cpu_stats(&self, handle: &VmHandle) -> Result<Vec<VcpuStats>, HypervisorError>;

    /// Query live guest memory statistics.
    async fn memory_stats(&self, handle: &VmHandle) -> Result<MemoryStats, HypervisorError>;

    /// Return `self` as `&dyn std::any::Any` for downcasting to concrete types.
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Detect the best available hypervisor backend.
///
/// Priority order — **native OS-level APIs first**, no third-party software needed:
///
/// 1. **WHP** (Windows) — Windows Hypervisor Platform. Same engine as Hyper-V.
///    Requires the "Virtual Machine Platform" optional Windows feature.
/// 2. **KVM** (Linux) — Kernel Virtual Machine. Built into the Linux kernel.
///    Available when `/dev/kvm` exists and is accessible.
/// 3. **AVF** (macOS) — Apple Virtualization Framework. Built into macOS 11+.
/// 4. **VirtualBox** — fallback if installed and VBoxManage.exe is found.
/// 5. **QEMU** — fallback if installed and qemu-system-x86_64 is found.
/// 6. **NullBackend** — no-op. Tells the user which feature to enable.
pub fn detect_backend() -> Arc<dyn HypervisorBackend> {
    // ── 1. Native OS hypervisor APIs ─────────────────────────────────────────
    #[cfg(target_os = "windows")]
    if let Some(whp) = backend::WhpBackend::detect() {
        tracing::info!("NovaVM-WHP backend: using Windows Hypervisor Platform (no third-party needed)");
        return Arc::new(whp);
    }

    #[cfg(target_os = "linux")]
    if let Some(kvm) = backend::KvmBackend::detect() {
        tracing::info!("NovaVM-KVM backend: using Linux KVM (no third-party needed)");
        return Arc::new(kvm);
    }

    #[cfg(target_os = "macos")]
    {
        tracing::info!("NovaVM-AVF backend: using Apple Virtualization Framework");
        return Arc::new(backend::AvfBackend::new());
    }

    // ── 2. Third-party hypervisors (optional, if installed) ──────────────────
    if let Some(vbox) = backend::VBoxBackend::detect() {
        tracing::info!("VirtualBox fallback backend selected");
        return Arc::new(vbox);
    }
    if let Some(qemu) = backend::QemuBackend::detect() {
        tracing::info!("QEMU fallback backend selected");
        return Arc::new(qemu);
    }

    // ── 3. Nothing found — give actionable instructions ──────────────────────
    #[cfg(target_os = "windows")]
    tracing::warn!(
        "Windows Hypervisor Platform not available. \
        Enable it with: Enable-WindowsOptionalFeature -Online -FeatureName VirtualMachinePlatform"
    );
    #[cfg(target_os = "linux")]
    tracing::warn!("KVM not available. Is /dev/kvm readable? Try: sudo chmod 666 /dev/kvm");

    Arc::new(backend::NullBackend)
}

#[cfg(test)]
mod tests {
    use super::*;
    use backend::NullBackend;

    #[tokio::test]
    async fn test_null_backend_capabilities() {
        let backend = NullBackend;
        let caps = backend.capabilities().await;
        assert_eq!(caps.backend_name, "null");
    }

    #[tokio::test]
    async fn test_null_backend_lifecycle() {
        let backend = NullBackend;
        let req = CreateVmRequest {
            name: "test".to_owned(),
            vcpus: 2,
            memory_mib: 512,
            firmware: crate::types::FirmwareType::Uefi,
            secure_boot: false,
            vtpm: false,
            disk_path: None,
            iso_path: None,
        };
        let handle = backend.create_vm(req).await.unwrap();
        backend.start_vm(&handle).await.unwrap();
        backend.pause_vm(&handle).await.unwrap();
        backend.resume_vm(&handle).await.unwrap();
        backend.stop_vm(&handle).await.unwrap();
        backend.destroy_vm(&handle).await.unwrap();
    }
}
