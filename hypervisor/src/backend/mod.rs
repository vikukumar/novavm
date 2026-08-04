//! Hypervisor backend implementations.
//!
//! Each backend is gated behind a `cfg` attribute so only the platform-native
//! implementation compiles on each target. The `NullBackend` always compiles
//! and is used for testing and unsupported platforms.

mod null;

#[cfg(target_os = "windows")]
mod whp;

#[cfg(target_os = "linux")]
mod kvm;

#[cfg(target_os = "macos")]
mod avf;

pub use null::NullBackend;

#[cfg(target_os = "windows")]
pub use whp::WhpBackend;

#[cfg(target_os = "linux")]
pub use kvm::KvmBackend;

#[cfg(target_os = "macos")]
pub use avf::AvfBackend;
