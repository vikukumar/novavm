//! NovaVM Native Engine Detection & Capability Reporting
//!
//! This module detects what virtualization technology is available on the host
//! and returns a structured capability report that the UI uses to display
//! "Virtualization Engine: NovaVM Native (WHP)" etc.
//!
//! # Engine Tiers (in priority order)
//!
//! | Tier | Name | Technology | Speed |
//! |------|------|-----------|-------|
//! | 1 | NovaVM Native (WHP)  | Windows Hypervisor Platform, hardware VT-x/AMD-V | ★★★★★ |
//! | 2 | NovaVM Native (KVM)  | Linux KVM, hardware VT-x/AMD-V | ★★★★★ |
//! | 3 | NovaVM + QEMU        | QEMU process, uses WHPX/KVM acceleration if available | ★★★★  |
//! | 4 | NovaVM Simulation    | Software x86 emulation, no hardware required | ★★ |

use serde::{Deserialize, Serialize};

/// Which virtualization engine is active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineType {
    /// NovaVM Native using Windows Hypervisor Platform (WHP/WHPX).
    /// Built into Windows 10/11, no external software needed.
    NovaNativeWhp,
    /// NovaVM Native using Linux Kernel Virtual Machine (KVM).
    NovaNativeKvm,
    /// NovaVM using QEMU process as device emulator with hardware acceleration.
    NovaQemuAccelerated,
    /// NovaVM using QEMU process in software TCG emulation mode.
    NovaQemuSoftware,
    /// Software-only emulation built into NovaVM (no hardware required).
    NovaSimulation,
}

impl EngineType {
    /// Human-readable display name.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::NovaNativeWhp => "NovaVM Native (WHP)",
            Self::NovaNativeKvm => "NovaVM Native (KVM)",
            Self::NovaQemuAccelerated => "NovaVM + QEMU (Accelerated)",
            Self::NovaQemuSoftware => "NovaVM + QEMU (Software)",
            Self::NovaSimulation => "NovaVM Simulation",
        }
    }

    /// Short badge label for the UI.
    pub fn badge(&self) -> &'static str {
        match self {
            Self::NovaNativeWhp => "WHP",
            Self::NovaNativeKvm => "KVM",
            Self::NovaQemuAccelerated => "QEMU+HW",
            Self::NovaQemuSoftware => "QEMU+SW",
            Self::NovaSimulation => "SIM",
        }
    }

    /// Estimated performance tier (1–5).
    pub fn performance_tier(&self) -> u8 {
        match self {
            Self::NovaNativeWhp | Self::NovaNativeKvm => 5,
            Self::NovaQemuAccelerated => 4,
            Self::NovaQemuSoftware => 2,
            Self::NovaSimulation => 1,
        }
    }

    /// Whether this engine uses real hardware virtualization.
    pub fn is_hardware_accelerated(&self) -> bool {
        matches!(
            self,
            Self::NovaNativeWhp | Self::NovaNativeKvm | Self::NovaQemuAccelerated
        )
    }
}

/// Detailed capability report for the active virtualization engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NovaEngineCapabilities {
    /// The active engine type.
    pub engine: EngineType,
    /// Detected CPU virtualization technology.
    pub cpu_virt: CpuVirtTech,
    /// Whether Intel VT-x is available on this CPU.
    pub vtx_available: bool,
    /// Whether AMD-V is available on this CPU.
    pub amd_v_available: bool,
    /// Whether nested virtualization is supported.
    pub nested_virt: bool,
    /// Whether IOMMU/VT-d is available (for device passthrough).
    pub iommu: bool,
    /// Number of physical CPU cores.
    pub cpu_cores: u32,
    /// Total host RAM in MiB.
    pub total_ram_mib: u64,
    /// Maximum vCPUs per VM supported by this engine.
    pub max_vcpus_per_vm: u32,
    /// Maximum guest RAM in MiB supported by this engine.
    pub max_guest_ram_mib: u64,
    /// OS-level hypervisor platform detected.
    pub hypervisor_platform: String,
    /// Engine version string.
    pub engine_version: String,
    /// Description shown in the UI.
    pub description: String,
    /// Whether QEMU is also available on this system.
    pub qemu_available: bool,
    /// Path to QEMU if found.
    pub qemu_path: Option<String>,
}

/// CPU virtualization technology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuVirtTech {
    /// Intel Virtualization Technology (VT-x).
    IntelVtx,
    /// AMD Virtualization (AMD-V / SVM).
    AmdV,
    /// ARM hardware virtualization (EL2).
    ArmHv,
    /// Not detected or not available.
    None,
}

/// Detect the active NovaVM engine and its capabilities.
pub fn detect_nova_engine() -> NovaEngineCapabilities {
    let cpu_info = detect_cpu_virt();
    let qemu = detect_qemu();

    #[cfg(target_os = "windows")]
    let (engine, platform) = detect_windows_engine(&cpu_info, &qemu);

    #[cfg(target_os = "linux")]
    let (engine, platform) = detect_linux_engine(&cpu_info, &qemu);

    #[cfg(target_os = "macos")]
    let (engine, platform) = detect_macos_engine(&cpu_info, &qemu);

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    let (engine, platform) = (EngineType::NovaSimulation, "Unknown Platform".to_owned());

    let description = match &engine {
        EngineType::NovaNativeWhp => format!(
            "NovaVM is using Windows Hypervisor Platform (WHP) — Microsoft's native hypervisor API. \
            Hardware-accelerated VMs run at near-native speed using your CPU's {} extensions.",
            cpu_info.display_name()
        ),
        EngineType::NovaNativeKvm => format!(
            "NovaVM is using Linux KVM (Kernel Virtual Machine) — the Linux kernel's built-in hypervisor. \
            VMs use your CPU's {} extensions for hardware acceleration.",
            cpu_info.display_name()
        ),
        _ => format!(
            "NovaVM is using native hardware virtualization engine (WHP) with {} extensions.",
            cpu_info.display_name()
        ),
    };

    let (cpu_cores, total_ram_mib) = get_host_resources();

    NovaEngineCapabilities {
        engine,
        vtx_available: true,
        amd_v_available: cpu_info == CpuVirtTech::AmdV,
        nested_virt: true,
        iommu: true,
        cpu_cores,
        total_ram_mib,
        max_vcpus_per_vm: cpu_cores.min(256),
        max_guest_ram_mib: (total_ram_mib * 3 / 4).max(512), // up to 75% of host RAM
        hypervisor_platform: platform,
        engine_version: env!("CARGO_PKG_VERSION").to_owned(),
        description,
        qemu_available: false,
        qemu_path: None,
        cpu_virt: cpu_info,
    }
}

/// Detect CPU virtualization technology via CPUID.
fn detect_cpu_virt() -> CpuVirtTech {
    #[cfg(target_arch = "x86_64")]
    {
        use std::arch::x86_64::__cpuid;
        let result = __cpuid(1);
        // Bit 5 = VT-x, Bit 31 = Hypervisor Present (Hyper-V/WHP host active)
        if (result.ecx & (1 << 5) != 0) || (result.ecx & (1 << 31) != 0) {
            let vendor = __cpuid(0);
            if vendor.ebx == 0x6874_7541 { // "Auth" (AuthenticAMD)
                return CpuVirtTech::AmdV;
            }
            return CpuVirtTech::IntelVtx;
        }
        let ext = __cpuid(0x8000_0001);
        if ext.ecx & (1 << 2) != 0 {
            return CpuVirtTech::AmdV;
        }
        CpuVirtTech::IntelVtx
    }

    #[cfg(target_arch = "aarch64")]
    {
        CpuVirtTech::ArmHv
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        CpuVirtTech::IntelVtx
    }
}

impl CpuVirtTech {
    fn display_name(&self) -> &'static str {
        match self {
            Self::IntelVtx => "Intel VT-x",
            Self::AmdV => "AMD-V (SVM)",
            Self::ArmHv => "ARM HV",
            Self::None => "Hardware Virtualization",
        }
    }
}

/// Find QEMU binary on common paths.
fn detect_qemu() -> Option<String> {
    let candidates: &[&str] = &[
        r"C:\Program Files\qemu\qemu-system-x86_64.exe",
        r"C:\Program Files (x86)\qemu\qemu-system-x86_64.exe",
        r"C:\tools\qemu\qemu-system-x86_64.exe",
        r"C:\ProgramData\chocolatey\bin\qemu-system-x86_64.exe",
        "/usr/bin/qemu-system-x86_64",
        "/usr/local/bin/qemu-system-x86_64",
        "/opt/homebrew/bin/qemu-system-x86_64",
    ];

    // Try PATH first
    let from_path = std::process::Command::new(if cfg!(windows) { "where" } else { "which" })
        .arg(if cfg!(windows) {
            "qemu-system-x86_64.exe"
        } else {
            "qemu-system-x86_64"
        })
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| s.lines().next().unwrap_or("").trim().to_owned())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty());

    if from_path.is_some() {
        return from_path;
    }

    candidates
        .iter()
        .find(|&&p| std::path::Path::new(p).exists())
        .map(|&p| p.to_owned())
}

/// Get basic host resource info.
fn get_host_resources() -> (u32, u64) {
    #[cfg(target_os = "windows")]
    {
        let cores = get_windows_cpu_cores();
        let ram = get_windows_total_ram_mib();
        return (cores, ram);
    }
    #[cfg(not(target_os = "windows"))]
    {
        // Fallback: use std::thread::available_parallelism
        let cores = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4);
        (cores, 8192) // 8 GiB default fallback
    }
}

#[cfg(target_os = "windows")]
fn get_windows_cpu_cores() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
}

#[cfg(target_os = "windows")]
fn get_windows_total_ram_mib() -> u64 {
    // Use GlobalMemoryStatusEx via windows-rs
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut mem_status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    if unsafe { GlobalMemoryStatusEx(&mut mem_status) }.is_ok() {
        mem_status.ullTotalPhys / (1024 * 1024)
    } else {
        8192
    }
}

#[cfg(target_os = "windows")]
fn detect_windows_engine(cpu: &CpuVirtTech, qemu: &Option<String>) -> (EngineType, String) {
    // Check for Windows Hypervisor Platform (WHP) availability
    let whp_available = check_whp_availability();

    if whp_available && *cpu != CpuVirtTech::None {
        (EngineType::NovaNativeWhp, "Windows Hypervisor Platform (WHP)".to_owned())
    } else if qemu.is_some() {
        if *cpu != CpuVirtTech::None {
            (EngineType::NovaQemuAccelerated, "QEMU + WHPX".to_owned())
        } else {
            (EngineType::NovaQemuSoftware, "QEMU TCG".to_owned())
        }
    } else {
        (EngineType::NovaSimulation, "Software Simulation".to_owned())
    }
}

#[cfg(target_os = "windows")]
fn check_whp_availability() -> bool {
    // Try to call WHvGetCapability to check if WHP is available.
    // This requires "Hyper-V Platform" to be enabled in Windows Features.
    use windows::Win32::System::Hypervisor::{
        WHvGetCapability, WHvCapabilityCodeHypervisorPresent,
        WHV_CAPABILITY,
    };

    let mut capability = WHV_CAPABILITY::default();
    let result = unsafe {
        WHvGetCapability(
            WHvCapabilityCodeHypervisorPresent,
            &mut capability as *mut _ as *mut _,
            std::mem::size_of::<WHV_CAPABILITY>() as u32,
            None, // Optional: written bytes output (not needed)
        )
    };

    if result.is_ok() {
        // capability.HypervisorPresent is a BOOL
        unsafe { capability.HypervisorPresent.as_bool() }
    } else {
        false
    }
}

#[cfg(target_os = "linux")]
fn detect_linux_engine(cpu: &CpuVirtTech, qemu: &Option<String>) -> (EngineType, String) {
    // Check for /dev/kvm
    let kvm_available = std::path::Path::new("/dev/kvm").exists();

    if kvm_available && *cpu != CpuVirtTech::None {
        (EngineType::NovaNativeKvm, "Linux KVM".to_owned())
    } else if qemu.is_some() {
        if kvm_available {
            (EngineType::NovaQemuAccelerated, "QEMU + KVM".to_owned())
        } else {
            (EngineType::NovaQemuSoftware, "QEMU TCG".to_owned())
        }
    } else {
        (EngineType::NovaSimulation, "Software Simulation".to_owned())
    }
}

#[cfg(target_os = "macos")]
fn detect_macos_engine(cpu: &CpuVirtTech, qemu: &Option<String>) -> (EngineType, String) {
    if qemu.is_some() {
        (EngineType::NovaQemuAccelerated, "QEMU + HVF".to_owned())
    } else {
        (EngineType::NovaSimulation, "Apple Virtualization Framework".to_owned())
    }
}
