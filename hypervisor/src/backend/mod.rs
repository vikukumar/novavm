//! Hypervisor backend implementations.
//!
//! Backend selection priority:
//! 1. QEMU process backend (cross-platform, actually runs real VMs) — preferred.
//! 2. Platform-native backends (WHP/KVM/AVF) — stubs, kept for future native implementation.
//! 3. NullBackend — no-op, used for testing only.

mod null;
mod qemu;

#[cfg(target_os = "windows")]
mod whp;

#[cfg(target_os = "linux")]
mod kvm;

#[cfg(target_os = "macos")]
mod avf;

pub use null::NullBackend;
pub use qemu::QemuBackend;

#[cfg(target_os = "windows")]
pub use whp::WhpBackend;

#[cfg(target_os = "linux")]
pub use kvm::KvmBackend;

#[cfg(target_os = "macos")]
pub use avf::AvfBackend;
