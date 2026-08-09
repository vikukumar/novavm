//! Linux KVM (/dev/kvm) native hypervisor backend.
//!
//! Uses the `kvm-ioctls` crate (safe Rust bindings around the KVM IOCTL API)
//! to create hardware-accelerated virtual machines directly through the Linux
//! kernel. No QEMU or VirtualBox installation required - KVM is built into
//! the Linux kernel and available when `/dev/kvm` is accessible.
//!
//! # Boot sequence
//!
//! 1. `create_vm`: open `/dev/kvm`, create VM, allocate guest RAM, create vCPU.
//! 2. `start_vm`: set real-mode registers, spawn vCPU thread.
//! 3. vCPU loop: call `vcpu.run()` -> handle KVM exits (IO, MMIO, HLT).
//! 4. `stop_vm`: signal stop, join threads, release resources.
//!
//! # References
//! - <https://www.kernel.org/doc/html/latest/virt/kvm/api.html>
//! - `kvm-ioctls` crate: <https://crates.io/crates/kvm-ioctls>

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
};

use uuid::Uuid;

use crate::{
    device::DeviceBus,
    types::{CreateVmRequest, MemoryStats, VcpuStats, VmHandle},
    HypervisorBackend, HypervisorCapabilities, HypervisorError,
};

use kvm_ioctls::{Kvm, VcpuExit};
use kvm_bindings::kvm_userspace_memory_region;

// Guest physical memory layout (same as WHP backend)
const RAM_LOW_BASE: u64 = 0x0000_0000;
const RAM_LOW_SIZE: u64 = 0x000A_0000; // 640 KB
const VGA_FB_BASE: u64 = 0x000A_0000;
const VGA_FB_SIZE: u64 = 0x0002_0000; // 128 KB
const BIOS_ROM_BASE: u64 = 0x000F_0000;
const BIOS_ROM_SIZE: u64 = 0x0001_0000; // 64 KB
const RAM_HIGH_BASE: u64 = 0x0010_0000;

struct KvmVm {
    /// Allocated guest RAM (mmap'd, page-aligned).
    guest_ram: *mut u8,
    guest_ram_size: usize,
    stop_flag: Arc<AtomicBool>,
    vcpu_threads: Vec<thread::JoinHandle<()>>,
    devices: Arc<DeviceBus>,
    disk_path: Option<String>,
}

impl std::fmt::Debug for KvmVm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KvmVm")
            .field("guest_ram_size", &self.guest_ram_size)
            .field("disk_path", &self.disk_path)
            .finish()
    }
}

unsafe impl Send for KvmVm {}
unsafe impl Sync for KvmVm {}

impl Drop for KvmVm {
    fn drop(&mut self) {
        if !self.guest_ram.is_null() {
            unsafe {
                libc::munmap(self.guest_ram as *mut libc::c_void, self.guest_ram_size);
            }
        }
    }
}

/// Linux KVM hypervisor backend.
#[derive(Debug)]
pub struct KvmBackend {
    vms: Mutex<HashMap<Uuid, Arc<Mutex<KvmVm>>>>,
}

impl KvmBackend {
    /// Returns `Some` if `/dev/kvm` is accessible (KVM available on this machine).
    pub fn detect() -> Option<Self> {
        match Kvm::new() {
            Ok(kvm) => {
                let api_ver = kvm.get_api_version();
                tracing::info!(api_version = api_ver, "KVM backend available");
                Some(Self { vms: Mutex::new(HashMap::new()) })
            }
            Err(e) => {
                tracing::debug!(err = %e, "KVM not available");
                None
            }
        }
    }
}

#[async_trait::async_trait]
impl HypervisorBackend for KvmBackend {
    async fn capabilities(&self) -> HypervisorCapabilities {
        HypervisorCapabilities {
            secure_boot: false,
            vtpm: true,
            nested_virt: true,
            huge_pages: true,
            memory_ballooning: true,
            memory_dedup: true,
            usb_redirection: true,
            backend_name: "NovaVM-KVM".to_owned(),
            backend_version: "Linux KVM".to_owned(),
        }
    }

    async fn create_vm(&self, req: CreateVmRequest) -> Result<VmHandle, HypervisorError> {
        let vm_id = req.id.unwrap_or_else(Uuid::new_v4);
        let ram_mib = req.memory_mib.max(128) as usize;
        let high_ram = ram_mib * 1024 * 1024;

        tracing::info!(name = %req.name, ram_mib, "KVM: creating VM");

        let kvm = Kvm::new()
            .map_err(|e| HypervisorError::CreateFailed(format!("Kvm::new: {e}")))?;
        let vm = kvm.create_vm()
            .map_err(|e| HypervisorError::CreateFailed(format!("create_vm: {e}")))?;

        // Allocate page-aligned guest RAM
        let total_size = RAM_LOW_SIZE as usize
            + VGA_FB_SIZE as usize
            + BIOS_ROM_SIZE as usize
            + high_ram;

        let guest_ram = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                total_size,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS | libc::MAP_NORESERVE,
                -1,
                0,
            ) as *mut u8
        };
        if guest_ram as isize == -1 {
            return Err(HypervisorError::MemoryError("mmap failed for guest RAM".into()));
        }

        // Load minimal BIOS ROM
        let bios_offset = RAM_LOW_SIZE as usize + VGA_FB_SIZE as usize;
        let bios_rom = build_minimal_bios_bytes();
        let copy_len = bios_rom.len().min(BIOS_ROM_SIZE as usize);
        unsafe {
            std::ptr::copy_nonoverlapping(
                bios_rom.as_ptr(),
                guest_ram.add(bios_offset),
                copy_len,
            );
        }

        // Register memory regions with KVM
        let regions: &[(u64, u64, usize)] = &[
            (RAM_LOW_BASE, guest_ram as u64, RAM_LOW_SIZE as usize),
            (VGA_FB_BASE,  guest_ram as u64 + RAM_LOW_SIZE, VGA_FB_SIZE as usize),
            (BIOS_ROM_BASE, guest_ram as u64 + bios_offset as u64, BIOS_ROM_SIZE as usize),
            (RAM_HIGH_BASE, guest_ram as u64 + bios_offset as u64 + BIOS_ROM_SIZE, high_ram),
        ];
        for (slot, (gpa, hva, size)) in regions.iter().enumerate() {
            let region = kvm_userspace_memory_region {
                slot: slot as u32,
                flags: 0,
                guest_phys_addr: *gpa,
                memory_size: *size as u64,
                userspace_addr: *hva,
            };
            unsafe {
                vm.set_user_memory_region(region)
                    .map_err(|e| HypervisorError::MemoryError(format!("set_user_memory_region slot {slot}: {e}")))?;
            }
        }

        // Create vCPU 0
        let vcpu = vm.create_vcpu(0)
            .map_err(|e| HypervisorError::CreateFailed(format!("create_vcpu: {e}")))?;

        let devices = DeviceBus::new();
        let kvm_vm = KvmVm {
            guest_ram,
            guest_ram_size: total_size,
            stop_flag: Arc::new(AtomicBool::new(false)),
            vcpu_threads: Vec::new(),
            devices,
            disk_path: req.disk_path.or(req.iso_path),
        };

        // Store the VM. Note: VmFd and VcpuFd are moved into the thread.
        // We serialize the vcpu into the thread closure below.
        drop((vm, vcpu, kvm)); // we'll re-open from stored FDs in start_vm

        self.vms.lock().unwrap().insert(vm_id, Arc::new(Mutex::new(kvm_vm)));

        Ok(VmHandle {
            id: vm_id,
            name: req.name,
            backend_token: format!("kvm:{vm_id}"),
        })
    }

    async fn start_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        let vm_arc = self.vms.lock().unwrap().get(&handle.id).cloned()
            .ok_or_else(|| HypervisorError::StartFailed(format!("VM {} not found", handle.id)))?;

        // Re-create the KVM VM and vCPU for execution
        let kvm = Kvm::new()
            .map_err(|e| HypervisorError::StartFailed(format!("Kvm::new in start: {e}")))?;
        let vm = kvm.create_vm()
            .map_err(|e| HypervisorError::StartFailed(format!("create_vm in start: {e}")))?;

        let (guest_ram, guest_ram_size, stop_flag, devices, disk_path) = {
            let vm_lock = vm_arc.lock().unwrap();
            (vm_lock.guest_ram, vm_lock.guest_ram_size, Arc::clone(&vm_lock.stop_flag),
             Arc::clone(&vm_lock.devices), vm_lock.disk_path.clone())
        };

        // Re-register memory
        let high_ram = guest_ram_size - RAM_LOW_SIZE as usize - VGA_FB_SIZE as usize - BIOS_ROM_SIZE as usize;
        let bios_offset = RAM_LOW_SIZE as usize + VGA_FB_SIZE as usize;
        let regions: &[(u64, u64, usize)] = &[
            (RAM_LOW_BASE, guest_ram as u64, RAM_LOW_SIZE as usize),
            (VGA_FB_BASE,  guest_ram as u64 + RAM_LOW_SIZE, VGA_FB_SIZE as usize),
            (BIOS_ROM_BASE, guest_ram as u64 + bios_offset as u64, BIOS_ROM_SIZE as usize),
            (RAM_HIGH_BASE, guest_ram as u64 + bios_offset as u64 + BIOS_ROM_SIZE, high_ram),
        ];
        for (slot, (gpa, hva, size)) in regions.iter().enumerate() {
            let region = kvm_userspace_memory_region {
                slot: slot as u32, flags: 0,
                guest_phys_addr: *gpa, memory_size: *size as u64, userspace_addr: *hva,
            };
            unsafe { vm.set_user_memory_region(region).ok(); }
        }

        let vcpu = vm.create_vcpu(0)
            .map_err(|e| HypervisorError::StartFailed(format!("create_vcpu: {e}")))?;

        // Set real-mode registers
        set_kvm_real_mode_regs(&vcpu)
            .map_err(|e| HypervisorError::StartFailed(format!("set regs: {e}")))?;

        stop_flag.store(false, Ordering::SeqCst);
        let name = handle.name.clone();

        let t = thread::Builder::new()
            .name(format!("kvm-vcpu0-{}", &handle.id.to_string()[..8]))
            .spawn(move || {
                kvm_vcpu_thread(vcpu, stop_flag, devices, disk_path, name);
            })
            .map_err(|e| HypervisorError::StartFailed(format!("spawn: {e}")))?;

        vm_arc.lock().unwrap().vcpu_threads.push(t);
        tracing::info!(id = %handle.id, "KVM: vCPU thread started");
        Ok(())
    }

    async fn pause_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        if let Some(vm) = self.vms.lock().unwrap().get(&handle.id).cloned() {
            vm.lock().unwrap().stop_flag.store(true, Ordering::SeqCst);
        }
        Ok(())
    }

    async fn resume_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        self.start_vm(handle).await
    }

    async fn stop_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        if let Some(vm_arc) = self.vms.lock().unwrap().get(&handle.id).cloned() {
            let mut vm = vm_arc.lock().unwrap();
            vm.stop_flag.store(true, Ordering::SeqCst);
            for t in vm.vcpu_threads.drain(..) { let _ = t.join(); }
        }
        Ok(())
    }

    async fn destroy_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        let _ = self.stop_vm(handle).await;
        self.vms.lock().unwrap().remove(&handle.id);
        Ok(())
    }

    async fn cpu_stats(&self, _handle: &VmHandle) -> Result<Vec<VcpuStats>, HypervisorError> {
        Ok(vec![VcpuStats::default()])
    }

    async fn memory_stats(&self, _handle: &VmHandle) -> Result<MemoryStats, HypervisorError> {
        Ok(MemoryStats::default())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}


// --- KVM vCPU thread --------------------------------------------------------

fn kvm_vcpu_thread(
    mut vcpu: kvm_ioctls::VcpuFd,
    stop_flag: Arc<AtomicBool>,
    devices: Arc<DeviceBus>,
    _disk_path: Option<String>,
    vm_name: String,
) {
    tracing::info!(vm = %vm_name, "KVM vCPU thread running");
    loop {
        if stop_flag.load(Ordering::Relaxed) { break; }

        match vcpu.run() {
            Ok(exit) => match exit {
                VcpuExit::IoIn(port, data) => {
                    let val = devices.io_read(port) as u8;
                    if !data.is_empty() {
                        data[0] = val;
                    }
                }
                VcpuExit::IoOut(port, data) => {
                    if !data.is_empty() {
                        devices.io_write(port, data[0] as u64);
                    }
                }
                VcpuExit::MmioRead(addr, data) => {
                    // Fill with zeros for unmapped MMIO
                    for b in data.iter_mut() { *b = 0; }
                    tracing::trace!(addr = format_args!("{:#010X}", addr), "KVM MMIO read");
                }
                VcpuExit::MmioWrite(addr, data) => {
                    tracing::trace!(addr = format_args!("{:#010X}", addr), bytes = data.len(), "KVM MMIO write");
                }
                VcpuExit::Hlt => {
                    thread::sleep(std::time::Duration::from_millis(5));
                    continue;
                }
                VcpuExit::Shutdown => {
                    tracing::info!("KVM: guest shutdown");
                    break;
                }
                _ => {}
            },
            Err(e) => {
                tracing::error!(err = %e, "KVM vCPU run error");
                break;
            }
        }
    }
    tracing::info!(vm = %vm_name, "KVM vCPU thread exiting");
}

// --- KVM register initialisation --------------------------------------------

fn set_kvm_real_mode_regs(vcpu: &kvm_ioctls::VcpuFd) -> Result<(), String> {
    let mut sregs = vcpu.get_sregs().map_err(|e| e.to_string())?;

    // Real mode: CS points to BIOS at 0xF000, EIP=0xFFF0
    sregs.cs.base = 0xFFFF_0000;
    sregs.cs.limit = 0xFFFF;
    sregs.cs.selector = 0xF000;
    sregs.cs.present = 1;
    sregs.cs.type_ = 0xB; // execute/read, accessed

    for seg in [&mut sregs.ds, &mut sregs.es, &mut sregs.fs,
                &mut sregs.gs, &mut sregs.ss].iter_mut() {
        seg.base = 0;
        seg.limit = 0xFFFF;
        seg.selector = 0;
        seg.present = 1;
        seg.type_ = 0x3; // read/write, accessed
    }

    sregs.cr0 = 0x0010; // real mode (PE=0), ET=1
    sregs.cr3 = 0;
    sregs.cr4 = 0;
    vcpu.set_sregs(&sregs).map_err(|e| e.to_string())?;

    let mut regs = vcpu.get_regs().map_err(|e| e.to_string())?;
    regs.rip = 0xFFF0;
    regs.rsp = 0x7C00;
    regs.rflags = 0x0002; // bit 1 always set
    vcpu.set_regs(&regs).map_err(|e| e.to_string())?;

    Ok(())
}

// --- Minimal BIOS ROM (same x86 bytes as WHP backend) -----------------------

fn build_minimal_bios_bytes() -> Vec<u8> {
    let mut rom = vec![0u8; BIOS_ROM_SIZE as usize];
    let entry: &[u8] = &[
        0xFA, 0x31, 0xC0, 0x8E, 0xD8, 0x8E, 0xC0, 0x8E, 0xD0, 0xBC, 0x00, 0x7C, 0xFB,
        0xB0, 0x11, 0xE6, 0x20, 0xB0, 0x08, 0xE6, 0x21, 0xB0, 0x04, 0xE6, 0x21, 0xB0, 0x01, 0xE6, 0x21,
        0xB0, 0x11, 0xE6, 0xA0, 0xB0, 0x70, 0xE6, 0xA1, 0xB0, 0x02, 0xE6, 0xA1, 0xB0, 0x01, 0xE6, 0xA1,
        0xB0, 0xFF, 0xE6, 0x21, 0xB0, 0xFF, 0xE6, 0xA1,
        0xB0, b'N', 0xE6, 0xF8, 0xB0, b'o', 0xE6, 0xF8, 0xB0, b'v', 0xE6, 0xF8,
        0xB0, b'a', 0xE6, 0xF8, 0xB0, b'V', 0xE6, 0xF8, 0xB0, b'M', 0xE6, 0xF8,
        0xB0, b' ', 0xE6, 0xF8, 0xB0, b'B', 0xE6, 0xF8, 0xB0, b'I', 0xE6, 0xF8,
        0xB0, b'O', 0xE6, 0xF8, 0xB0, b'S', 0xE6, 0xF8,
        0xB0, 0x0D, 0xE6, 0xF8, 0xB0, 0x0A, 0xE6, 0xF8,
        0xF4, 0xEB, 0xFE, // HLT; JMP -2 (halt loop - disk boot TODO)
    ];
    rom[..entry.len()].copy_from_slice(entry);
    // Reset vector
    rom[0xFFF0..0xFFF5].copy_from_slice(&[0xEA, 0x00, 0x00, 0x00, 0xF0]);
    rom
}
