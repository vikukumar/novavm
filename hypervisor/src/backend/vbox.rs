//! VirtualBox backend for NovaVM.
//!
//! Uses `VBoxManage.exe` CLI to create, start, pause, stop and destroy real
//! VirtualBox virtual machines. When a VM is started with [`start_vm`], VirtualBox
//! opens its own full graphical display window  this gives the user the real
//! VMware-like experience with full OS installer support.
//!
//! # Requirements
//! - VirtualBox must be installed. `VBoxManage.exe` is looked up on standard
//!   paths and in the system PATH.
//!
//! # Disk formats
//! VirtualBox supports VDI, VMDK, and VHD natively.
//! QCOW2 is NOT supported by VirtualBox  a warning is logged if one is provided.

use std::{
    collections::HashMap,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
};

use uuid::Uuid;

use crate::{
    types::{CreateVmRequest, FirmwareType, MemoryStats, VcpuStats, VmHandle},
    HypervisorBackend, HypervisorCapabilities, HypervisorError,
};

type VmNameTable = Arc<Mutex<HashMap<Uuid, String>>>;

/// VirtualBox hypervisor backend using VBoxManage.exe.
#[derive(Debug, Clone)]
pub struct VBoxBackend {
    vboxmanage: PathBuf,
    vm_names: VmNameTable,
}

impl VBoxBackend {
    /// Detect VBoxManage.exe and return a backend instance, or None if not found.
    pub fn detect() -> Option<Self> {
        let path = Self::find_vboxmanage()?;
        tracing::info!(path = %path.display(), "VirtualBox backend detected");
        Some(Self {
            vboxmanage: path,
            vm_names: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    fn find_vboxmanage() -> Option<PathBuf> {
        let candidates = [
            r"C:\Program Files\Oracle\VirtualBox\VBoxManage.exe",
            r"C:\Program Files (x86)\Oracle\VirtualBox\VBoxManage.exe",
            r"C:\tools\VirtualBox\VBoxManage.exe",
        ];
        if let Ok(out) = Command::new("where").arg("VBoxManage.exe").output() {
            if out.status.success() {
                let s = String::from_utf8_lossy(&out.stdout);
                let first = s.lines().next().unwrap_or("").trim();
                if !first.is_empty() {
                    return Some(PathBuf::from(first));
                }
            }
        }
        for c in &candidates {
            let p = PathBuf::from(c);
            if p.exists() {
                return Some(p);
            }
        }
        None
    }

    fn vboxmanage(&self, args: &[&str]) -> Result<String, String> {
        let out = Command::new(&self.vboxmanage)
            .args(args)
            .output()
            .map_err(|e| format!("Failed to run VBoxManage: {e}"))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&out.stderr).to_string())
        }
    }

    fn guess_os_type(name: &str) -> &'static str {
        let lower = name.to_lowercase();
        if lower.contains("win11") { "Windows11_64" }
        else if lower.contains("win10") || lower.contains("windows") { "Windows10_64" }
        else if lower.contains("ubuntu") { "Ubuntu_64" }
        else if lower.contains("debian") { "Debian_64" }
        else if lower.contains("fedora") { "Fedora_64" }
        else if lower.contains("arch") { "ArchLinux_64" }
        else { "Linux_64" }
    }
}

#[async_trait::async_trait]
impl HypervisorBackend for VBoxBackend {
    async fn capabilities(&self) -> HypervisorCapabilities {
        HypervisorCapabilities {
            secure_boot: true,
            vtpm: true,
            nested_virt: true,
            huge_pages: false,
            memory_ballooning: false,
            memory_dedup: false,
            usb_redirection: true,
            backend_name: "VirtualBox".to_owned(),
            backend_version: "7.x".to_owned(),
        }
    }

    async fn create_vm(&self, req: CreateVmRequest) -> Result<VmHandle, HypervisorError> {
        tracing::info!(
            name = %req.name, vcpus = req.vcpus, memory_mib = req.memory_mib,
            disk = ?req.disk_path, iso = ?req.iso_path,
            "VBoxBackend: creating VM"
        );
        let os_type = Self::guess_os_type(&req.name);
        let vm_name = req.name.clone();

        // 1. Create and register VM
        self.vboxmanage(&["createvm", "--name", &vm_name, "--ostype", os_type, "--register"])
            .map_err(|e| HypervisorError::CreateFailed(format!("createvm: {e}")))?;

        // 2. Set CPU / RAM / firmware / display / network
        let memory_str = req.memory_mib.to_string();
        let vcpu_str = req.vcpus.to_string();
        let fw = match req.firmware { FirmwareType::Uefi => "efi64", FirmwareType::Bios => "bios" };
        self.vboxmanage(&[
            "modifyvm", &vm_name,
            "--cpus", &vcpu_str, "--memory", &memory_str, "--firmware", fw,
            "--graphicscontroller", "vmsvga", "--vram", "128",
            "--audio-driver", "default", "--audio-enabled", "on",
            "--nic1", "nat", "--nictype1", "virtio",
            "--usb", "on", "--usbehci", "on",
            "--clipboard-mode", "bidirectional",
        ]).map_err(|e| HypervisorError::CreateFailed(format!("modifyvm: {e}")))?;

        // 3. SATA controller for hard disk
        self.vboxmanage(&[
            "storagectl", &vm_name, "--name", "SATA Controller",
            "--add", "sata", "--controller", "IntelAhci", "--portcount", "4", "--bootable", "on",
        ]).map_err(|e| HypervisorError::CreateFailed(format!("storagectl SATA: {e}")))?;

        // 4. IDE controller for DVD/ISO
        self.vboxmanage(&[
            "storagectl", &vm_name, "--name", "IDE Controller", "--add", "ide",
        ]).map_err(|e| HypervisorError::CreateFailed(format!("storagectl IDE: {e}")))?;

        // 5. Attach hard disk (skip QCOW2 - not supported by VirtualBox)
        if let Some(ref disk_path) = req.disk_path {
            let ext = std::path::Path::new(disk_path)
                .extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if ext == "qcow2" {
                tracing::warn!(disk = %disk_path,
                    "VirtualBox does not support QCOW2. Disk not attached. Use VDI/VMDK/VHD.");
            } else {
                self.vboxmanage(&[
                    "storageattach", &vm_name, "--storagectl", "SATA Controller",
                    "--port", "0", "--device", "0", "--type", "hdd", "--medium", disk_path,
                ]).map_err(|e| HypervisorError::CreateFailed(format!("attach disk: {e}")))?;
                tracing::info!(disk = %disk_path, "VBoxBackend: hard disk attached");
            }
        }

        // 6. Attach ISO as DVD and set boot order
        if let Some(ref iso_path) = req.iso_path {
            self.vboxmanage(&[
                "storageattach", &vm_name, "--storagectl", "IDE Controller",
                "--port", "0", "--device", "0", "--type", "dvddrive", "--medium", iso_path,
            ]).map_err(|e| HypervisorError::CreateFailed(format!("attach ISO: {e}")))?;
            // Boot from DVD first
            self.vboxmanage(&[
                "modifyvm", &vm_name,
                "--boot1", "dvd", "--boot2", "disk", "--boot3", "none", "--boot4", "none",
            ]).ok();
            tracing::info!(iso = %iso_path, "VBoxBackend: ISO attached, boot order DVD first");
        } else {
            self.vboxmanage(&[
                "modifyvm", &vm_name,
                "--boot1", "disk", "--boot2", "none", "--boot3", "none", "--boot4", "none",
            ]).ok();
        }

        let handle_id = Uuid::new_v4();
        self.vm_names.lock().expect("vm_names poisoned").insert(handle_id, vm_name.clone());
        tracing::info!(name = %vm_name, id = %handle_id, "VBoxBackend: VM created and registered");

        Ok(VmHandle {
            id: handle_id,
            name: vm_name.clone(),
            backend_token: vm_name,
        })
    }

    /// Start the VM  opens the real VirtualBox GUI window with full display.
    async fn start_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        let vm_name = &handle.backend_token;
        tracing::info!(name = %vm_name, "VBoxBackend: starting VM (GUI mode)");
        self.vboxmanage(&["startvm", vm_name, "--type", "gui"])
            .map_err(|e| HypervisorError::StartFailed(format!("startvm '{vm_name}': {e}")))?;
        tracing::info!(name = %vm_name, "VBoxBackend: VirtualBox GUI window opened");
        Ok(())
    }

    async fn pause_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        self.vboxmanage(&["controlvm", &handle.backend_token, "pause"])
            .map_err(|e| HypervisorError::PauseFailed(e))?;
        Ok(())
    }

    async fn resume_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        self.vboxmanage(&["controlvm", &handle.backend_token, "resume"])
            .map_err(|e| HypervisorError::ResumeFailed(e))?;
        Ok(())
    }

    async fn stop_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        let vm_name = &handle.backend_token;
        if self.vboxmanage(&["controlvm", vm_name, "acpipowerbutton"]).is_err() {
            let _ = self.vboxmanage(&["controlvm", vm_name, "poweroff"]);
        }
        Ok(())
    }

    async fn destroy_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        let vm_name = &handle.backend_token;
        tracing::info!(name = %vm_name, "VBoxBackend: destroying VM");
        let _ = self.vboxmanage(&["controlvm", vm_name, "poweroff"]);
        self.vboxmanage(&["unregistervm", vm_name, "--delete"])
            .map_err(|e| HypervisorError::DestroyFailed(format!("unregistervm '{vm_name}': {e}")))?;
        self.vm_names.lock().expect("vm_names poisoned").remove(&handle.id);
        tracing::info!(name = %vm_name, "VBoxBackend: VM destroyed");
        Ok(())
    }

    async fn cpu_stats(&self, handle: &VmHandle) -> Result<Vec<VcpuStats>, HypervisorError> {
        let out = self.vboxmanage(&["metrics", "query", &handle.backend_token, "Guest/CPU/Load/User"]);
        let pct = out.ok().as_deref().map(parse_metric_percent).unwrap_or(0.0);
        Ok(vec![VcpuStats { index: 0, guest_percent: pct, hypervisor_percent: 0.0, idle_percent: (100.0 - pct).max(0.0) }])
    }

    async fn memory_stats(&self, handle: &VmHandle) -> Result<MemoryStats, HypervisorError> {
        let out = self.vboxmanage(&["metrics", "query", &handle.backend_token, "Guest/RAM/Usage/Total"]);
        let total_mib = out.ok().as_deref().map(parse_metric_kb).unwrap_or(0) / 1024;
        Ok(MemoryStats { total_mib, used_mib: total_mib, available_mib: 0, balloon_size_mib: 0 })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}


fn parse_metric_percent(text: &str) -> f64 {
    for line in text.lines() {
        if let Some(idx) = line.rfind('%') {
            if let Some(v) = line[..idx].trim().split_whitespace().last().and_then(|s| s.parse::<f64>().ok()) {
                return v;
            }
        }
    }
    0.0
}

fn parse_metric_kb(text: &str) -> u64 {
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if let Some(v) = parts.iter().rev().nth(1).and_then(|s| s.parse::<u64>().ok()) {
            return v;
        }
    }
    0
}