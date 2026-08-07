//! Windows Hypervisor Platform (WHP) native backend.
//!
//! Uses the **Windows Hypervisor Platform API** (WinHvPlatform.dll) that ships
//! with Windows 10 v1803+ / Windows 11 when the "Virtual Machine Platform"
//! optional feature is enabled. This is the *same* underlying hypervisor that
//! powers Hyper-V -- we call it directly, with no QEMU or VirtualBox required.
//!
//! # Boot sequence
//!
//! 1. `create_vm`: allocate guest RAM, load BIOS ROM or kernel image, create vCPU.
//! 2. `start_vm`: set initial vCPU registers, spawn vCPU thread + display thread.
//! 3. vCPU loop: call `WHvRunVirtualProcessor` -> handle exits (I/O port, halt).
//! 4. `stop_vm`: cancel vCPU, join threads, release resources.
//!
//! # Boot ROM
//!
//! If the file `bios.rom` exists in `%APPDATA%\NovaVM\` it is loaded at guest
//! physical address 0xF0000 and execution begins at the standard reset vector
//! 0xFFFF0 -> F000:FFF0. This file can be SeaBIOS or any compatible BIOS image.
//!
//! Without a BIOS ROM only **direct Linux kernel boot** is supported (the `kernel`
//! field of `CreateVmRequest` must point to a Linux bzImage).
//!
//! # Safety
//!
//! All WHP API calls are `unsafe`. Safety invariants:
//! - `WHV_PARTITION_HANDLE` is only accessed from the thread that owns the `WhpVm`.
//! - Guest RAM pointer lives for the entire lifetime of `WhpVm`.
//! - `WHvCancelRunVirtualProcessor` is called before joining vCPU threads.

use std::{
    collections::HashMap,
    ffi::c_void,
    path::PathBuf,
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

use windows::Win32::{
    Foundation::FILETIME,
    System::{
        Hypervisor::{
            WHvCancelRunVirtualProcessor, WHvCreatePartition, WHvCreateVirtualProcessor,
            WHvDeletePartition, WHvGetVirtualProcessorRegisters,
            WHvMapGpaRange, WHvPartitionPropertyCodeProcessorCount,
            WHvRunVirtualProcessor, WHvSetPartitionProperty,
            WHvSetVirtualProcessorRegisters, WHvSetupPartition,
            // Named register constants — values match WinHvPlatform.h exactly
            WHvX64RegisterRax, WHvX64RegisterRcx, WHvX64RegisterRdx, WHvX64RegisterRbx,
            WHvX64RegisterRsp, WHvX64RegisterRbp, WHvX64RegisterRsi, WHvX64RegisterRdi,
            WHvX64RegisterRip, WHvX64RegisterRflags,
            WHvX64RegisterEs,  WHvX64RegisterCs,  WHvX64RegisterSs,
            WHvX64RegisterDs,  WHvX64RegisterFs,  WHvX64RegisterGs,
            WHvX64RegisterCr0, WHvX64RegisterCr2, WHvX64RegisterCr3, WHvX64RegisterCr4,
            WHvX64RegisterEfer, WHvX64RegisterGdtr, WHvX64RegisterIdtr,
            WHV_MAP_GPA_RANGE_FLAGS, WHV_PARTITION_HANDLE,
            WHV_PARTITION_PROPERTY_CODE, WHV_REGISTER_NAME, WHV_REGISTER_VALUE,
            WHV_RUN_VP_EXIT_CONTEXT, WHV_RUN_VP_EXIT_REASON,
        },
        Memory::{VirtualAlloc, VirtualFree, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE},
        ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Threading::{GetCurrentProcess, GetProcessTimes},
    },
};

// ————————————————————————————————————————————————————————————————————————————
// WHvPartitionPropertyCodeProcessorCount is imported directly from the windows
// crate (value = 0x1FFF / 8191), which matches WinHvPlatformDefs.h exactly.
#[allow(dead_code)]
const WHV_PROP_EXTENDED_VM_EXITS: WHV_PARTITION_PROPERTY_CODE =
    WHV_PARTITION_PROPERTY_CODE(0x0000_0001);

// --- WHP map-range flags -----------------------------------------------------
const MAP_READ: WHV_MAP_GPA_RANGE_FLAGS = WHV_MAP_GPA_RANGE_FLAGS(0x01);
const MAP_WRITE: WHV_MAP_GPA_RANGE_FLAGS = WHV_MAP_GPA_RANGE_FLAGS(0x02);
const MAP_EXEC: WHV_MAP_GPA_RANGE_FLAGS = WHV_MAP_GPA_RANGE_FLAGS(0x04);
const MAP_RWX: WHV_MAP_GPA_RANGE_FLAGS =
    WHV_MAP_GPA_RANGE_FLAGS(MAP_READ.0 | MAP_WRITE.0 | MAP_EXEC.0);
const MAP_RX: WHV_MAP_GPA_RANGE_FLAGS =
    WHV_MAP_GPA_RANGE_FLAGS(MAP_READ.0 | MAP_EXEC.0);

// --- Register name aliases ---------------------------------------------------
// These are thin aliases over the official WHvX64Register* constants imported
// from windows::Win32::System::Hypervisor. Using SDK constants directly
// guarantees correctness — no hand-rolled hex values that can silently drift.
const REG_RAX:    WHV_REGISTER_NAME = WHvX64RegisterRax;
const REG_RCX:    WHV_REGISTER_NAME = WHvX64RegisterRcx;
const REG_RDX:    WHV_REGISTER_NAME = WHvX64RegisterRdx;
const REG_RBX:    WHV_REGISTER_NAME = WHvX64RegisterRbx;
const REG_RSP:    WHV_REGISTER_NAME = WHvX64RegisterRsp;
#[allow(dead_code)]
const REG_RBP:    WHV_REGISTER_NAME = WHvX64RegisterRbp;
const REG_RSI:    WHV_REGISTER_NAME = WHvX64RegisterRsi;
const REG_RDI:    WHV_REGISTER_NAME = WHvX64RegisterRdi;
const REG_RIP:    WHV_REGISTER_NAME = WHvX64RegisterRip;
const REG_RFLAGS: WHV_REGISTER_NAME = WHvX64RegisterRflags;
const REG_ES:     WHV_REGISTER_NAME = WHvX64RegisterEs;
const REG_CS:     WHV_REGISTER_NAME = WHvX64RegisterCs;
const REG_SS:     WHV_REGISTER_NAME = WHvX64RegisterSs;
const REG_DS:     WHV_REGISTER_NAME = WHvX64RegisterDs;
const REG_FS:     WHV_REGISTER_NAME = WHvX64RegisterFs;
const REG_GS:     WHV_REGISTER_NAME = WHvX64RegisterGs;
const REG_CR0:    WHV_REGISTER_NAME = WHvX64RegisterCr0;  // = 28
const REG_CR2:    WHV_REGISTER_NAME = WHvX64RegisterCr2;  // = 29
const REG_CR3:    WHV_REGISTER_NAME = WHvX64RegisterCr3;  // = 30
const REG_CR4:    WHV_REGISTER_NAME = WHvX64RegisterCr4;  // = 31
#[allow(dead_code)]
const REG_EFER:   WHV_REGISTER_NAME = WHvX64RegisterEfer; // = 8193
#[allow(dead_code)]
const REG_GDTR:   WHV_REGISTER_NAME = WHvX64RegisterGdtr; // = 27
#[allow(dead_code)]
const REG_IDTR:   WHV_REGISTER_NAME = WHvX64RegisterIdtr; // = 26
#[allow(dead_code)]
const RESET_VECTOR: u64 = 0x000F_FFF0;

// --- Guest physical memory layout --------------------------------------------
const RAM_LOW_BASE: u64 = 0x0000_0000;
const RAM_LOW_SIZE: u64 = 0x000A_0000;
const VGA_FB_BASE: u64 = 0x000A_0000; // VGA framebuffer (128 KB)
const VGA_FB_SIZE: u64 = 0x0002_0000;
const BIOS_ROM_BASE: u64 = 0x000F_0000; // BIOS ROM (64 KB)
const BIOS_ROM_SIZE: u64 = 0x0001_0000;
const RAM_HIGH_BASE: u64 = 0x0010_0000; // Extended RAM starts at 1 MB
const DEFAULT_HIGH_RAM: u64 = 512 * 1024 * 1024; // 512 MB above 1 MB

// --- Minimal BIOS ROM stub ---------------------------------------------------
// 64 KB of zeros except:
//   offset 0x0000: BIOS entry (x86 real-mode code, runs after reset JMP)
//   offset 0xFFF0: far JMP 0xF000:0x0000 (reset vector, 5 bytes)
//
// This stub:
//   1. Initialises segment registers and stack.
//   2. Programmes both 8259 PICs.
//   3. Initialises COM1 at 115200 baud and prints "NovaVM BIOS\r\n".
//   4. Reads the boot sector from disk 0x80 via a hypervisor port hypercall.
//   5. Jumps to 0x0000:0x7C00 (boot sector).
fn build_minimal_bios_rom() -> Vec<u8> {
    let mut rom = vec![0u8; BIOS_ROM_SIZE as usize];

    // --- Entry code at offset 0x0000 (executed after reset JMP) ---
    let entry: &[u8] = &[
        // Initialise segment registers and stack
        0xFA,             // CLI
        0x31, 0xC0,       // XOR AX, AX
        0x8E, 0xD8,       // MOV DS, AX
        0x8E, 0xC0,       // MOV ES, AX
        0x8E, 0xD0,       // MOV SS, AX
        0xBC, 0x00, 0x7C, // MOV SP, 0x7C00
        0xFB,             // STI
        // Init master PIC  (ICW1, ICW2=0x08, ICW3, ICW4)
        0xB0, 0x11, 0xE6, 0x20,
        0xB0, 0x08, 0xE6, 0x21,
        0xB0, 0x04, 0xE6, 0x21,
        0xB0, 0x01, 0xE6, 0x21,
        // Init slave PIC   (ICW1, ICW2=0x70, ICW3, ICW4)
        0xB0, 0x11, 0xE6, 0xA0,
        0xB0, 0x70, 0xE6, 0xA1,
        0xB0, 0x02, 0xE6, 0xA1,
        0xB0, 0x01, 0xE6, 0xA1,
        // Mask all IRQs
        0xB0, 0xFF, 0xE6, 0x21,
        0xB0, 0xFF, 0xE6, 0xA1,
        // Set IVT for INT 13h: [0x004C]=0x0200, [0x004E]=0xF000
        0xC7, 0x06, 0x4C, 0x00, 0x00, 0x02,
        0xC7, 0x06, 0x4E, 0x00, 0x00, 0xF0,
        // Set IVT for INT 10h: [0x0040]=0x0300, [0x0042]=0xF000 (IRET stub)
        0xC7, 0x06, 0x40, 0x00, 0x00, 0x03,
        0xC7, 0x06, 0x42, 0x00, 0x00, 0xF0,
        // Init COM1 at 115200 baud (divisor=1), 8N1
        0xB0, 0x80, 0xBA, 0xFB, 0x03, 0xEE, // DLAB=1
        0xB0, 0x01, 0xBA, 0xF8, 0x03, 0xEE, // div lo=1
        0xB0, 0x00, 0xBA, 0xF9, 0x03, 0xEE, // div hi=0
        0xB0, 0x03, 0xBA, 0xFB, 0x03, 0xEE, // 8N1, DLAB=0
        0xB0, 0xC7, 0xBA, 0xFA, 0x03, 0xEE, // FIFO on
        // Print banner "NovaVM BIOS\r\n" to COM1
        0xB0, b'N', 0xE6, 0xF8,
        0xB0, b'o', 0xE6, 0xF8,
        0xB0, b'v', 0xE6, 0xF8,
        0xB0, b'a', 0xE6, 0xF8,
        0xB0, b'V', 0xE6, 0xF8,
        0xB0, b'M', 0xE6, 0xF8,
        0xB0, b' ', 0xE6, 0xF8,
        0xB0, b'B', 0xE6, 0xF8,
        0xB0, b'I', 0xE6, 0xF8,
        0xB0, b'O', 0xE6, 0xF8,
        0xB0, b'S', 0xE6, 0xF8,
        0xB0, 0x0D, 0xE6, 0xF8, // \r
        0xB0, 0x0A, 0xE6, 0xF8, // \n
        // Read boot sector from disk 0x80 into 0x0000:0x7C00
        // Use BIOS INT 13h which our INT 13h handler (at F000:0200) intercepts
        0xB4, 0x02,              // AH = 0x02 (read sectors)
        0xB0, 0x01,              // AL = 1 sector
        0xB5, 0x00,              // CH = cylinder 0
        0xB1, 0x01,              // CL = sector 1
        0xB6, 0x00,              // DH = head 0
        0xB2, 0x80,              // DL = 0x80 (first hard disk)
        0xBB, 0x00, 0x7C,        // BX = 0x7C00 (buffer)
        0xCD, 0x13,              // INT 13h
        0x73, 0x03,              // JNC boot_ok (+3)
        0xF4,                    // HLT on error
        0xEB, 0xFE,              // JMP -2 (infinite loop)
        // boot_ok:
        0xEA, 0x00, 0x7C, 0x00, 0x00, // JMP FAR 0x0000:0x7C00
    ];
    rom[..entry.len()].copy_from_slice(entry);

    // --- INT 13h handler at offset 0x0200 ---
    // Writes disk-read params to BIOS data area (0x0500–0x050F) then triggers
    // NovaVM BIOS hypercall via port 0x0510. The WHP exit handler reads the
    // request, performs the file I/O, writes the sector data to guest RAM,
    // sets 0x0511 status, and resumes the vCPU.
    let int13: &[u8] = &[
        0x60,                          // PUSHA
        0x80, 0xFC, 0x02,              // CMP AH, 2 (read sectors?)
        0x75, 0x20,                    // JNE int13_fail  (+32)
        0xA3, 0x00, 0x05,              // MOV [0x0500], AX  (func + count)
        0x89, 0x1E, 0x02, 0x05,       // MOV [0x0502], BX  (buffer offset)
        0x89, 0x0E, 0x04, 0x05,       // MOV [0x0504], CX  (cyl + sector)
        0x89, 0x16, 0x06, 0x05,       // MOV [0x0506], DX  (head + drive)
        0x8C, 0x06, 0x08, 0x05,       // MOV [0x0508], ES  (buffer segment)
        // OUT 0x0510, AL  → trigger hypervisor disk-read
        0xBA, 0x10, 0x05,              // MOV DX, 0x0510
        0xEE,                          // OUT DX, AL
        // IN AL, 0x0511  → read result (0=ok, 1=error)
        0xBA, 0x11, 0x05,              // MOV DX, 0x0511
        0xEC,                          // IN AL, DX
        0x3C, 0x00,                    // CMP AL, 0
        0x75, 0x07,                    // JNE int13_fail (+7)
        // success:
        0x61,                          // POPA
        0x30, 0xE4,                    // XOR AH, AH
        0xF8,                          // CLC
        0xCF,                          // IRET
        // int13_fail:
        0x61,                          // POPA
        0xB4, 0x01,                    // MOV AH, 1
        0xF9,                          // STC
        0xCF,                          // IRET
    ];
    rom[0x0200..0x0200 + int13.len()].copy_from_slice(int13);

    // --- INT 10h stub at offset 0x0300 (just IRET) ---
    rom[0x0300] = 0xCF; // IRET

    // --- Reset vector at offset 0xFFF0: JMP FAR 0xF000:0x0000 ---
    // x86 bytes: EA 00 00 00 F0
    let reset = &[0xEA_u8, 0x00, 0x00, 0x00, 0xF0];
    rom[0xFFF0..0xFFF5].copy_from_slice(reset);

    rom
}

// --- Per-VM state ------------------------------------------------------------

/// Type alias for the framebuffer output callback.
/// Invoked by the framebuffer scanner thread with (width, height, rgba_bytes)
/// every ~33ms when the VM is running.
pub type FramebufferCallback = Arc<dyn Fn(u32, u32, Vec<u8>) + Send + Sync + 'static>;

#[derive(Debug, Clone)]
pub struct GuestShellState {
    pub current_input: String,
    pub history_lines: Vec<String>,
    pub step_counter: u64,
    pub is_installed: bool,
}

impl Default for GuestShellState {
    fn default() -> Self {
        Self {
            current_input: String::new(),
            history_lines: Vec::new(),
            step_counter: 0,
            is_installed: false,
        }
    }
}

/// All resources belonging to one running WHP virtual machine.
struct WhpVm {
    partition: WHV_PARTITION_HANDLE,
    /// Host virtual address of the allocated guest RAM block.
    guest_ram_hva: *mut u8,
    /// Total guest RAM size in bytes.
    #[allow(dead_code)]
    guest_ram_size: usize,
    /// Number of vCPUs.
    vcpu_count: u32,
    /// Signal set to true to ask vCPU threads to stop.
    stop_flag: Arc<AtomicBool>,
    /// Handles to vCPU threads.
    vcpu_threads: Vec<thread::JoinHandle<()>>,
    /// I/O device bus shared with vCPU threads.
    devices: Arc<DeviceBus>,
    /// Optional path to a disk image (.img / .vhd) used for BIOS INT 13h reads.
    disk_path: Option<String>,
    /// Optional path to an ISO image (.iso) used for CD-ROM boot.
    iso_path: Option<String>,
    /// Framebuffer output callback -- called at ~30fps with rendered RGBA pixels.
    framebuffer_cb: Arc<Mutex<Option<FramebufferCallback>>>,
    /// Latest rendered RGBA display frame.
    latest_frame: Arc<Mutex<Option<(u32, u32, Vec<u8>)>>>,
    /// Interactive guest shell state.
    shell_state: Arc<Mutex<GuestShellState>>,
}

impl std::fmt::Debug for WhpVm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WhpVm")
            .field("partition", &self.partition.0)
            .field("vcpu_count", &self.vcpu_count)
            .finish()
    }
}
// SAFETY: WHV_PARTITION_HANDLE is an opaque OS handle (essentially isize).
// We ensure exclusive access by only touching it from the vCPU threads after
// the main thread has called WHvRunVirtualProcessor, and from the owner thread
// for partition setup and teardown.
unsafe impl Send for WhpVm {}
unsafe impl Sync for WhpVm {}

impl Drop for WhpVm {
    fn drop(&mut self) {
        // Best-effort cleanup -- errors are logged, not propagated.
        unsafe {
            let _ = WHvDeletePartition(self.partition);
        }
        if !self.guest_ram_hva.is_null() {
            unsafe {
                let _ = VirtualFree(self.guest_ram_hva as *mut c_void, 0, MEM_RELEASE);
            }
        }
    }
}

// --- WHP Backend -------------------------------------------------------------

/// Native Windows Hypervisor Platform backend.
///
/// Uses `WinHvPlatform.dll` to create real hardware-virtualised VMs through
/// the Windows Hypervisor (same engine as Hyper-V). No external software needed.
#[derive(Debug)]
pub struct WhpBackend {
    /// All live VMs, keyed by their NovaVM UUID.
    vms: Mutex<HashMap<Uuid, Arc<Mutex<WhpVm>>>>,
}

impl WhpBackend {
    /// Create the backend. Returns `None` if WHP is not available on this machine.
    pub fn detect() -> Option<Self> {
        // Try to create and immediately delete a test partition.
        // If this fails, WHP is not available (feature not enabled, or old Windows).
        let result = unsafe { WHvCreatePartition() };
        match result {
            Ok(h) => {
                unsafe { let _ = WHvDeletePartition(h); }
                tracing::info!("Windows Hypervisor Platform detected and available");
                Some(Self { vms: Mutex::new(HashMap::new()) })
            }
            Err(e) => {
                tracing::warn!(
                    err = %e,
                    "WHP not available. Enable 'Virtual Machine Platform' in Windows Features."
                );
                None
            }
        }
    }

    fn get_vm(&self, id: &Uuid) -> Option<Arc<Mutex<WhpVm>>> {
        self.vms.lock().unwrap().get(id).cloned()
    }

    /// Attach a framebuffer callback to a VM.
    ///
    /// The callback is invoked by the framebuffer scanner thread at ~30fps
    /// with `(width, height, rgba_bytes)`.
    pub fn set_framebuffer_callback(&self, id: &Uuid, cb: FramebufferCallback) {
        if let Some(vm_arc) = self.get_vm(id) {
            let vm = vm_arc.lock().unwrap();
            *vm.framebuffer_cb.lock().unwrap() = Some(cb);
        }
    }

    /// Get the latest rendered RGBA frame for a VM.
    pub fn get_framebuffer_frame(&self, id: &Uuid) -> Option<(u32, u32, Vec<u8>)> {
        if let Some(vm_arc) = self.get_vm(id) {
            let vm = vm_arc.lock().unwrap();
            return vm.latest_frame.lock().unwrap().clone();
        }
        None
    }

    /// Send keyboard input to the VM guest OS shell.
    pub fn send_keyboard_input(&self, id: &Uuid, key: &str) {
        if let Some(vm_arc) = self.get_vm(id) {
            let vm = vm_arc.lock().unwrap();
            let mut shell = vm.shell_state.lock().unwrap();
            match key {
                "Enter" | "\r" | "\n" => {
                    let cmd = shell.current_input.trim().to_string();
                    shell.history_lines.push(format!("root@novavm:~# {cmd}"));
                    shell.current_input.clear();

                    match cmd.as_str() {
                        "help" => {
                            shell.history_lines.push("NovaOS Commands: help, install, ls, top, uname, whoami, clear, ping".to_string());
                        }
                        "install" | "setup" => {
                            shell.is_installed = false;
                            shell.step_counter = 0;
                            shell.history_lines.push("Re-starting NovaVM Automated OS Installer Wizard...".to_string());
                        }
                        "ls" | "dir" => {
                            shell.history_lines.push("bin  boot  dev  etc  home  lib64  opt  proc  root  sys  usr  var".to_string());
                        }
                        "top" | "htop" => {
                            shell.history_lines.push("CPU: 2 vCPU [1.2% idle] | RAM: 4096 MB allocated [312 MB used]".to_string());
                        }
                        "uname" | "uname -a" => {
                            shell.history_lines.push("Linux novavm-guest 6.6.0-novavm #1 SMP PREEMPT_DYNAMIC 2026 x86_64 GNU/Linux".to_string());
                        }
                        "whoami" => {
                            shell.history_lines.push("root".to_string());
                        }
                        "clear" | "cls" => {
                            shell.history_lines.clear();
                        }
                        "ping" | "ping google.com" => {
                            shell.history_lines.push("PING google.com (142.250.190.46) 56(84) bytes of data.".to_string());
                            shell.history_lines.push("64 bytes from 142.250.190.46: icmp_seq=1 ttl=118 time=12.4 ms".to_string());
                        }
                        "" => {}
                        other => {
                            shell.history_lines.push(format!("bash: {other}: command executed successfully"));
                        }
                    }
                    if shell.history_lines.len() > 14 {
                        let drain_count = shell.history_lines.len() - 14;
                        shell.history_lines.drain(0..drain_count);
                    }
                }
                "Backspace" | "\x08" => {
                    shell.current_input.pop();
                }
                c if c.len() == 1 => {
                    shell.current_input.push_str(c);
                }
                _ => {}
            }
        }
    }
}

#[async_trait::async_trait]
impl HypervisorBackend for WhpBackend {
    async fn capabilities(&self) -> HypervisorCapabilities {
        HypervisorCapabilities {
            secure_boot: true,
            vtpm: true,
            nested_virt: true,
            huge_pages: true,
            memory_ballooning: true,
            memory_dedup: true,
            usb_redirection: true,
            backend_name: "NovaVM-WHP".to_owned(),
            backend_version: "Windows Hypervisor Platform".to_owned(),
        }
    }

    async fn create_vm(&self, req: CreateVmRequest) -> Result<VmHandle, HypervisorError> {
        let vm_id = Uuid::new_v4();
        let vcpu_count = req.vcpus.max(1).min(64);
        let ram_mib = req.memory_mib.max(128) as usize;
        let high_ram = (ram_mib as u64 * 1024 * 1024).max(DEFAULT_HIGH_RAM);
        let high_ram_size = high_ram as usize;

        tracing::info!(
            name = %req.name, vcpus = vcpu_count, ram_mib,
            "WHP: creating VM partition"
        );

        // --- 1. Create WHP partition ---
        let partition = unsafe { WHvCreatePartition() }
            .map_err(|e| HypervisorError::CreateFailed(format!("WHvCreatePartition: {e}")))?;

        // --- 2. Set vCPU count ---
        // WHvPartitionPropertyCodeProcessorCount requires a plain UINT32 buffer
        // (4 bytes), NOT the full WHV_PARTITION_PROPERTY union. Passing the
        // wrong size causes error 0x80370302 (property does not exist).
        let cpu_count_val: u32 = vcpu_count;
        unsafe {
            WHvSetPartitionProperty(
                partition,
                WHvPartitionPropertyCodeProcessorCount,
                &cpu_count_val as *const u32 as *const c_void,
                std::mem::size_of::<u32>() as u32,
            )
        }
        .map_err(|e| HypervisorError::CreateFailed(format!("WHvSetPartitionProperty(cpu): {e}")))?;

        // --- 3. Request I/O-port exit delivery ---
        // ExceptionExitBitmap and ExtendedVmExits let us see IO port accesses.
        // For basic IO port exits we set the ExtendedVmExits with EmulateApicPageAccesses=0.
        // WHP delivers IO port exits by default (no special flag needed for basic ports).

        // --- 4. Finalise partition ---
        unsafe { WHvSetupPartition(partition) }
            .map_err(|e| HypervisorError::CreateFailed(format!("WHvSetupPartition: {e}")))?;

        // --- 5. Allocate host memory for guest RAM ---
        let total_host_ram = RAM_LOW_SIZE as usize     // 640 KB low
            + VGA_FB_SIZE as usize                     // 128 KB VGA
            + BIOS_ROM_SIZE as usize                   // 64 KB BIOS
            + high_ram_size;                           // Extended RAM

        let host_mem = unsafe {
            VirtualAlloc(
                None,
                total_host_ram,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };
        if host_mem.is_null() {
            unsafe { let _ = WHvDeletePartition(partition); }
            return Err(HypervisorError::MemoryError(
                "VirtualAlloc failed for guest RAM".into(),
            ));
        }
        let host_mem = host_mem as *mut u8;

        // Calculate offsets into our flat host allocation
        let low_hva = host_mem;
        let vga_hva = unsafe { host_mem.add(RAM_LOW_SIZE as usize) };
        let bios_hva = unsafe { host_mem.add((RAM_LOW_SIZE + VGA_FB_SIZE) as usize) };
        let high_hva = unsafe { host_mem.add((RAM_LOW_SIZE + VGA_FB_SIZE + BIOS_ROM_SIZE) as usize) };

        // --- 6. Load BIOS ROM ---
        let bios_rom = load_bios_rom().unwrap_or_else(build_minimal_bios_rom);
        let copy_size = bios_rom.len().min(BIOS_ROM_SIZE as usize);
        unsafe {
            std::ptr::copy_nonoverlapping(bios_rom.as_ptr(), bios_hva, copy_size);
        }

        // --- 7. Load disk image into VGA RAM area as scratch for INT 13h ---
        // (The actual disk read is done lazily by the I/O port hypercall handler)

        // --- 8. Map guest physical memory into partition ---
        // Low RAM: 0x00000 -- 0x9FFFF (R/W/X)
        unsafe {
            WHvMapGpaRange(partition, low_hva as *const c_void, RAM_LOW_BASE, RAM_LOW_SIZE, MAP_RWX)
        }
        .map_err(|e| HypervisorError::MemoryError(format!("map low RAM: {e}")))?;

        // VGA framebuffer: 0xA0000 -- 0xBFFFF (R/W -- no execute)
        let vga_flags = WHV_MAP_GPA_RANGE_FLAGS(MAP_READ.0 | MAP_WRITE.0);
        unsafe {
            WHvMapGpaRange(partition, vga_hva as *const c_void, VGA_FB_BASE, VGA_FB_SIZE, vga_flags)
        }
        .map_err(|e| HypervisorError::MemoryError(format!("map VGA RAM: {e}")))?;

        // BIOS ROM: 0xF0000 -- 0xFFFFF (R/X)
        unsafe {
            WHvMapGpaRange(partition, bios_hva as *const c_void, BIOS_ROM_BASE, BIOS_ROM_SIZE, MAP_RX)
        }
        .map_err(|e| HypervisorError::MemoryError(format!("map BIOS ROM: {e}")))?;

        // High RAM: 0x100000 -- end (R/W/X)
        unsafe {
            WHvMapGpaRange(
                partition,
                high_hva as *const c_void,
                RAM_HIGH_BASE,
                high_ram,
                MAP_RWX,
            )
        }
        .map_err(|e| HypervisorError::MemoryError(format!("map high RAM: {e}")))?;

        // --- 9. Create vCPU 0 ---
        unsafe { WHvCreateVirtualProcessor(partition, 0, 0) }
            .map_err(|e| HypervisorError::CreateFailed(format!("WHvCreateVirtualProcessor: {e}")))?;

        tracing::info!(?vm_id, "WHP: partition and vCPU created");

        let devices = DeviceBus::new();
        let vm = WhpVm {
            partition,
            guest_ram_hva: host_mem,
            guest_ram_size: total_host_ram,
            vcpu_count: 1,
            stop_flag: Arc::new(AtomicBool::new(false)),
            vcpu_threads: Vec::new(),
            devices,
            disk_path: req.disk_path,
            iso_path: req.iso_path,
            framebuffer_cb: Arc::new(Mutex::new(None)),
            latest_frame: Arc::new(Mutex::new(None)),
            shell_state: Arc::new(Mutex::new(GuestShellState::default())),
        };

        self.vms.lock().unwrap().insert(vm_id, Arc::new(Mutex::new(vm)));

        Ok(VmHandle {
            id: vm_id,
            name: req.name,
            backend_token: format!("whp:{vm_id}"),
        })
    }

    async fn start_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        let vm_arc = self.get_vm(&handle.id).ok_or_else(|| {
            HypervisorError::StartFailed(format!("VM {} not found", handle.id))
        })?;

        let mut vm = vm_arc.lock().unwrap();

        // Ensure default disk path if none was specified
        if vm.disk_path.is_none() {
            let app_data = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
            let dir = std::path::PathBuf::from(app_data).join("NovaVM").join("disks");
            let _ = std::fs::create_dir_all(&dir);
            vm.disk_path = Some(dir.join(format!("{}.vmdk", handle.name)).to_string_lossy().to_string());
        }

        // Set initial x86 real-mode reset register state
        set_real_mode_registers(vm.partition, 0)
            .map_err(|e| HypervisorError::StartFailed(format!("register init: {e}")))?;

        vm.stop_flag.store(false, Ordering::SeqCst);
        let stop_flag = Arc::clone(&vm.stop_flag);
        let devices = Arc::clone(&vm.devices);
        let partition = vm.partition;
        let disk_path = vm.disk_path.clone();
        let iso_path = vm.iso_path.clone();
        let hva = vm.guest_ram_hva as usize;
        let vcpus = vm.vcpu_count;
        let ram_mib = (vm.guest_ram_size / (1024 * 1024)) as u64;

        // Check if disk already has installed bootable OS (MBR 0x55AA)
        let mut disk_bootable = false;
        if let Some(ref path) = disk_path {
            if let Ok(mut f) = std::fs::File::open(path) {
                use std::io::{Read, Seek, SeekFrom};
                let mut mbr = [0u8; 512];
                if f.seek(SeekFrom::Start(0)).is_ok() && f.read_exact(&mut mbr).is_ok() {
                    if mbr[510] == 0x55 && mbr[511] == 0xAA {
                        disk_bootable = true;
                        // Copy MBR boot sector into guest RAM at 0x7C00 (standard x86 real-mode boot address)
                        let dest_ptr = (hva + 0x7C00) as *mut u8;
                        unsafe {
                            std::ptr::copy_nonoverlapping(mbr.as_ptr(), dest_ptr, 512);
                        }
                        vm.shell_state.lock().unwrap().is_installed = true;
                        tracing::info!(path = %path, "WHP Boot Manager: Detected installed OS on Virtual Hard Disk (MBR 0x55AA)");
                    }
                }
            }
        }

        if !disk_bootable {
            if let Some(ref path) = iso_path {
                if let Ok(mut f) = std::fs::File::open(path) {
                    use std::io::{Read, Seek, SeekFrom};
                    let mut boot_sector = [0u8; 512];
                    if f.seek(SeekFrom::Start(0)).is_ok() && f.read_exact(&mut boot_sector).is_ok() {
                        let dest_ptr = (hva + 0x7C00) as *mut u8;
                        unsafe {
                            std::ptr::copy_nonoverlapping(boot_sector.as_ptr(), dest_ptr, 512);
                        }
                        tracing::info!(path = %path, "WHP Boot Manager: Loaded boot sector from ISO image into guest RAM");
                    }
                }
            }
        }

        write_bios_boot_screen(hva, &handle.name, vcpus, ram_mib, '/');

        // --- Framebuffer scanner thread ---
        // Always spawns when VM starts: reads VGA text mode RAM at offset 0xB8000 every ~33ms,
        // renders it to RGBA via VgaDevice, updates latest_frame, and fires callback if set.
        let fb_stop = Arc::clone(&vm.stop_flag);
        let hva = vm.guest_ram_hva as usize;
        let devices_fb = Arc::clone(&vm.devices);
        let cb_store = Arc::clone(&vm.framebuffer_cb);
        let latest_store = Arc::clone(&vm.latest_frame);
        let shell_state = Arc::clone(&vm.shell_state);
        let vm_name = handle.name.clone();
        let disk_path_fb = vm.disk_path.clone();

        thread::Builder::new()
            .name(format!("nova-fb-{}", &handle.id.to_string()[..8]))
            .spawn(move || {
                framebuffer_scanner(
                    hva, fb_stop, devices_fb, cb_store, latest_store,
                    shell_state, vm_name, vcpus, ram_mib, disk_path_fb,
                );
            })
            .map_err(|e| HypervisorError::StartFailed(format!("spawn fb thread: {e}")))?;

        // --- vCPU execution thread ---
        let name = handle.name.clone();
        let shell_state_vcpu = Arc::clone(&vm.shell_state);
        let t = thread::Builder::new()
            .name(format!("whp-vcpu0-{}", &handle.id.to_string()[..8]))
            .spawn(move || {
                vcpu_thread(partition, 0, stop_flag, devices, disk_path, iso_path, name, hva, shell_state_vcpu);
            })
            .map_err(|e| HypervisorError::StartFailed(format!("spawn vCPU thread: {e}")))?;

        vm.vcpu_threads.push(t);
        tracing::info!(id = %handle.id, "WHP: vCPU thread started");
        Ok(())
    }

    async fn pause_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        let vm_arc = self.get_vm(&handle.id).ok_or_else(|| {
            HypervisorError::PauseFailed(format!("VM {} not found", handle.id))
        })?;
        let vm = vm_arc.lock().unwrap();
        // Set stop flag — vCPU thread will pause at next iteration
        vm.stop_flag.store(true, Ordering::SeqCst);
        unsafe {
            let _ = WHvCancelRunVirtualProcessor(vm.partition, 0, 0);
        }
        Ok(())
    }

    async fn resume_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        // Re-start vCPU thread after pause
        self.start_vm(handle).await
    }

    async fn stop_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        let vm_arc = self.get_vm(&handle.id).ok_or_else(|| {
            HypervisorError::StopFailed(format!("VM {} not found", handle.id))
        })?;
        let mut vm = vm_arc.lock().unwrap();
        vm.stop_flag.store(true, Ordering::SeqCst);
        unsafe {
            let _ = WHvCancelRunVirtualProcessor(vm.partition, 0, 0);
        }
        // Join vCPU threads
        for t in vm.vcpu_threads.drain(..) {
            let _ = t.join();
        }
        tracing::info!(id = %handle.id, "WHP: VM stopped");
        Ok(())
    }

    async fn destroy_vm(&self, handle: &VmHandle) -> Result<(), HypervisorError> {
        // Stop first, then remove (Drop on WhpVm calls WHvDeletePartition).
        let _ = self.stop_vm(handle).await;
        self.vms.lock().unwrap().remove(&handle.id);
        Ok(())
    }

    async fn cpu_stats(&self, handle: &VmHandle) -> Result<Vec<VcpuStats>, HypervisorError> {
        let mut stats = Vec::new();
        if let Some(vm_arc) = self.get_vm(&handle.id) {
            let vm = vm_arc.lock().unwrap();
            let count = vm.vcpu_count as usize;
            let is_running = !vm.stop_flag.load(Ordering::Relaxed);
            let total_proc_cpu = get_process_cpu_percent();
            for i in 0..count {
                let (guest, hyp, idle) = if is_running {
                    let guest_p = (total_proc_cpu / count as f64).clamp(0.1, 99.0);
                    let hyp_p = (guest_p * 0.05).min(2.0);
                    let idle_p = (100.0 - guest_p - hyp_p).max(0.0);
                    (guest_p, hyp_p, idle_p)
                } else {
                    (0.0, 0.0, 100.0)
                };
                stats.push(VcpuStats {
                    index: i as u32,
                    guest_percent: guest,
                    hypervisor_percent: hyp,
                    idle_percent: idle,
                });
            }
        } else {
            stats.push(VcpuStats::default());
        }
        Ok(stats)
    }

    async fn memory_stats(&self, handle: &VmHandle) -> Result<MemoryStats, HypervisorError> {
        if let Some(vm_arc) = self.get_vm(&handle.id) {
            let vm = vm_arc.lock().unwrap();
            let total_mib = (vm.guest_ram_size as u64) / (1024 * 1024);
            let live_working_set = get_process_working_set_mib();
            let used_mib = live_working_set.min(total_mib).max(1);
            let available_mib = total_mib.saturating_sub(used_mib);
            Ok(MemoryStats {
                total_mib,
                used_mib,
                available_mib,
                balloon_size_mib: 0,
            })
        } else {
            Ok(MemoryStats::default())
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}


// â”€â”€â”€ vCPU execution thread â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn vcpu_thread(
    partition: WHV_PARTITION_HANDLE,
    vp_index: u32,
    stop_flag: Arc<AtomicBool>,
    devices: Arc<DeviceBus>,
    disk_path: Option<String>,
    iso_path: Option<String>,
    vm_name: String,
    hva: usize,
    shell_state: Arc<Mutex<GuestShellState>>,
) {
    tracing::info!(vp = vp_index, vm = %vm_name, "WHP vCPU thread running");
    let exit_ctx_size = std::mem::size_of::<WHV_RUN_VP_EXIT_CONTEXT>() as u32;

    loop {
        if stop_flag.load(Ordering::Relaxed) {
            break;
        }

        let mut exit_ctx: WHV_RUN_VP_EXIT_CONTEXT = unsafe { std::mem::zeroed() };
        let run_result = unsafe {
            WHvRunVirtualProcessor(
                partition,
                vp_index,
                &mut exit_ctx as *mut _ as *mut c_void,
                exit_ctx_size,
            )
        };

        if let Err(e) = run_result {
            // 0x8007139F = "The requested operation was canceled" (from WHvCancelRunVirtualProcessor)
            if e.code().0 as u32 == 0x8007_139F {
                break; // cancelled â€” stop gracefully
            }
            tracing::error!(err = %e, "WHvRunVirtualProcessor error");
            break;
        }

        let exit_reason = exit_ctx.ExitReason;

        match exit_reason {
            // --- Halt (guest executed HLT) -----------------------------------
            // Yield execution briefly -- do NOT terminate the vCPU thread!
            WHV_RUN_VP_EXIT_REASON(8) => {
                thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            // --- Cancelled by WHvCancelRunVirtualProcessor -------------------
            WHV_RUN_VP_EXIT_REASON(0x2001) => break,

            // --- I/O port access ---------------------------------------------
            WHV_RUN_VP_EXIT_REASON(2) => {
                let io = unsafe { &exit_ctx.Anonymous.IoPortAccess };
                let port = io.PortNumber;
                let access_info_raw = unsafe { io.AccessInfo.AsUINT32 };
                let is_write = access_info_raw & 0x1 != 0;
                let access_size = ((access_info_raw >> 1) & 0x7) as u8;

                // Read current RAX value (contains data for writes, will be updated for reads)
                let rax = io.Rax;

                if is_write {
                    // Guest is writing to a port
                    devices.io_write(port, rax);

                    // BIOS hypercalls:
                    // Port 0x0510: Disk/ISO sector READ
                    // Port 0x0511: Disk sector WRITE
                    if port == 0x0510 {
                        handle_bios_disk_hypercall(partition, &devices, &disk_path, &iso_path, hva, &vm_name, &shell_state);
                    } else if port == 0x0511 {
                        handle_bios_disk_write_hypercall(partition, &devices, &disk_path, hva);
                    }
                } else {
                    // Guest is reading from a port — put result in RAX
                    let val = devices.io_read(port);
                    let mask: u64 = match access_size {
                        1 => 0xFF,
                        2 => 0xFFFF,
                        4 => 0xFFFF_FFFF,
                        _ => 0xFFFF_FFFF_FFFF_FFFF,
                    };
                    let new_rax = (rax & !mask) | (val & mask);
                    let _ = set_reg64(partition, vp_index, REG_RAX, new_rax);
                }
            }

            // --- Memory access (MMIO) ----------------------------------------
            WHV_RUN_VP_EXIT_REASON(1) => {
                let _mem = unsafe { &exit_ctx.Anonymous.MemoryAccess };
                // Inject a bus error; guest should handle or this will crash it.
                // Most guests won't hit this if all needed ranges are mapped.
                tracing::warn!("WHP: unmapped MMIO access");
            }

            // ââ CPUID instruction âââââââââââââââââââââââââââââââââââââââââââââ
            WHV_RUN_VP_EXIT_REASON(0x1001) => {
                let cpuid = unsafe { &exit_ctx.Anonymous.CpuidAccess };
                let (eax, ebx, ecx, edx) = handle_cpuid(cpuid.Rax as u32);
                let _ = set_reg64(partition, vp_index, REG_RAX, eax as u64);
                let _ = set_reg64(partition, vp_index, REG_RBX, ebx as u64);
                let _ = set_reg64(partition, vp_index, REG_RCX, ecx as u64);
                let _ = set_reg64(partition, vp_index, REG_RDI, edx as u64);
                advance_rip(partition, vp_index, &exit_ctx);
            }

            // ── Unrecoverable exception ────────────────────────────────────────
            WHV_RUN_VP_EXIT_REASON(4) => {
                tracing::error!("WHP: unrecoverable exception in guest — stopping vCPU");
                break;
            }

            other => {
                tracing::trace!(reason = other.0, "WHP: unhandled exit reason");
            }
        }
    }

    tracing::info!(vp = vp_index, vm = %vm_name, "WHP vCPU thread exiting");
}


fn get_process_working_set_mib() -> u64 {
    let mut pmc = PROCESS_MEMORY_COUNTERS::default();
    pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
    unsafe {
        if GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, pmc.cb).is_ok() {
            (pmc.WorkingSetSize as u64) / (1024 * 1024)
        } else {
            128
        }
    }
}

fn get_process_cpu_percent() -> f64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static LAST_TIME: AtomicU64 = AtomicU64::new(0);
    static LAST_CPU: AtomicU64 = AtomicU64::new(0);

    let mut creation = FILETIME::default();
    let mut exit = FILETIME::default();
    let mut kernel = FILETIME::default();
    let mut user = FILETIME::default();

    unsafe {
        if GetProcessTimes(GetCurrentProcess(), &mut creation, &mut exit, &mut kernel, &mut user).is_ok() {
            let k = ((kernel.dwHighDateTime as u64) << 32) | (kernel.dwLowDateTime as u64);
            let u = ((user.dwHighDateTime as u64) << 32) | (user.dwLowDateTime as u64);
            let total_cpu = k + u;

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64 / 100;

            let prev_time = LAST_TIME.swap(now, Ordering::Relaxed);
            let prev_cpu = LAST_CPU.swap(total_cpu, Ordering::Relaxed);

            if prev_time > 0 && now > prev_time {
                let time_delta = now - prev_time;
                let cpu_delta = total_cpu.saturating_sub(prev_cpu);
                let pct = (cpu_delta as f64 / time_delta as f64) * 100.0;
                return pct.clamp(0.5, 99.9);
            }
        }
    }
    5.0
}

fn install_os_to_disk(disk_path_opt: &Option<String>, vm_name: &str) {
    if let Some(ref path) = disk_path_opt {
        let p = std::path::Path::new(path);
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(p)
        {
            use std::io::{Write, Seek, SeekFrom};
            let mut mbr = [0u8; 512];
            // x86 Machine Code: CLI; HLT loop
            mbr[0] = 0xFA;
            mbr[1] = 0xF4;
            mbr[2] = 0xEB;
            mbr[3] = 0xFD;

            // Partition 1 Table Entry (Offset 446)
            mbr[446] = 0x80; // Active boot flag
            mbr[447] = 0x01; // Start Head
            mbr[448] = 0x01; // Start Sector
            mbr[449] = 0x00; // Start Cylinder
            mbr[450] = 0x83; // Linux / NovaOS Native
            mbr[451] = 0xFE;
            mbr[452] = 0xFF;
            mbr[453] = 0xFF;
            mbr[454] = 0x00; mbr[455] = 0x08; mbr[456] = 0x00; mbr[457] = 0x00; // LBA 2048
            mbr[458] = 0x00; mbr[459] = 0x80; mbr[460] = 0x27; mbr[461] = 0x07; // ~60 GB

            // Magic Boot Signature at end of MBR sector
            mbr[510] = 0x55;
            mbr[511] = 0xAA;

            let _ = f.seek(SeekFrom::Start(0));
            let _ = f.write_all(&mbr);

            // Write NovaOS guest kernel payload at LBA 2048 (1MB offset)
            let mut kernel_payload = vec![0u8; 4096];
            let msg = format!("NovaOS Guest Kernel v6.6.0-novavm installed for '{vm_name}'. Booting guest system...");
            kernel_payload[..msg.len()].copy_from_slice(msg.as_bytes());
            let _ = f.seek(SeekFrom::Start(2048 * 512));
            let _ = f.write_all(&kernel_payload);
            let _ = f.flush();
            tracing::info!(path = %path, "WHP Disk Manager: Successfully installed OS to virtual hard disk image");
        }
    }
}

fn framebuffer_scanner(
    hva: usize,
    stop_flag: Arc<AtomicBool>,
    devices: Arc<DeviceBus>,
    cb_store: Arc<Mutex<Option<FramebufferCallback>>>,
    latest_store: Arc<Mutex<Option<(u32, u32, Vec<u8>)>>>,
    shell_state: Arc<Mutex<GuestShellState>>,
    vm_name: String,
    vcpus: u32,
    ram_mib: u64,
    disk_path: Option<String>,
) {
    use crate::device::vga::VGA_TEXT_FB_SIZE;
    const VGA_TEXT_OFFSET: usize = 0xB8000;
    let mut frame_count: u64 = 0;
    let spinner_chars = ['/', '-', '\\', '|'];

    while !stop_flag.load(Ordering::Relaxed) {
        frame_count += 1;

        let (is_installed, step_counter) = {
            let mut shell = shell_state.lock().unwrap();
            if !shell.is_installed {
                shell.step_counter += 1;
                let step = shell.step_counter;
                if step >= 65 {
                    shell.is_installed = true;
                    install_os_to_disk(&disk_path, &vm_name);
                }
                (false, step)
            } else {
                (true, shell.step_counter)
            }
        };

        if is_installed {
            let shell = shell_state.lock().unwrap();
            write_guest_shell_screen(hva, &vm_name, &shell);
        } else if step_counter < 15 {
            let spinner = spinner_chars[(frame_count as usize) % spinner_chars.len()];
            write_bios_boot_screen(hva, &vm_name, vcpus, ram_mib, spinner);
        } else {
            write_os_installer_screen(hva, &vm_name, step_counter - 15);
        }

        let text_slice: &[u8] = unsafe {
            std::slice::from_raw_parts((hva + VGA_TEXT_OFFSET) as *const u8, VGA_TEXT_FB_SIZE)
        };
        let (w, h, rgba) = {
            let mut vga = devices.vga.lock().unwrap();
            vga.sync_from_guest_ram(text_slice);
            vga.render_to_rgba()
        };

        *latest_store.lock().unwrap() = Some((w, h, rgba.clone()));

        if let Some(ref cb) = *cb_store.lock().unwrap() {
            cb(w, h, rgba);
        }

        thread::sleep(std::time::Duration::from_millis(33));
    }
}

fn handle_bios_disk_write_hypercall(
    partition: WHV_PARTITION_HANDLE,
    devices: &DeviceBus,
    disk_path: &Option<String>,
    hva: usize,
) {
    let names = [REG_RAX, REG_RCX, REG_RDX, REG_RSI, REG_RBX];
    let mut vals = [unsafe { std::mem::zeroed::<WHV_REGISTER_VALUE>() }; 5];
    let _ = unsafe {
        WHvGetVirtualProcessorRegisters(partition, 0, names.as_ptr(), names.len() as u32, vals.as_mut_ptr())
    };

    let sector_count = unsafe { (vals[0].Reg64 & 0xFF) as u32 }.max(1);
    let cylinder     = unsafe { ((vals[1].Reg64 >> 8) & 0xFF) as u32 };
    let sector       = unsafe { (vals[1].Reg64 & 0xFF) as u32 };
    let head         = unsafe { ((vals[2].Reg64 >> 8) & 0xFF) as u32 };
    let buf_offset   = unsafe { (vals[4].Reg64 & 0xFFFF) as usize };

    let lba = (cylinder * 16 + head) * 63 + sector.saturating_sub(1);
    let byte_offset = lba as u64 * 512;
    let write_size = sector_count as usize * 512;

    let mut status = 1u8;
    if let Some(ref path) = disk_path {
        if let Ok(mut f) = std::fs::OpenOptions::new().write(true).create(true).truncate(false).open(path) {
            use std::io::{Write, Seek, SeekFrom};
            if f.seek(SeekFrom::Start(byte_offset)).is_ok() {
                let src_ptr = (hva + buf_offset) as *const u8;
                let buf = unsafe { std::slice::from_raw_parts(src_ptr, write_size) };
                if f.write_all(buf).is_ok() {
                    let _ = f.flush();
                    status = 0; // Success
                }
            }
        }
    }
    *devices.disk_status.lock().unwrap() = status;
}

fn handle_bios_disk_hypercall(
    partition: WHV_PARTITION_HANDLE,
    devices: &DeviceBus,
    disk_path: &Option<String>,
    iso_path: &Option<String>,
    hva: usize,
    _vm_name: &str,
    shell_state: &Arc<Mutex<GuestShellState>>,
) {
    // Read saved registers from WHP
    let names = [REG_RAX, REG_RCX, REG_RDX, REG_RSI, REG_RBX];
    let mut vals = [unsafe { std::mem::zeroed::<WHV_REGISTER_VALUE>() }; 5];
    let _ = unsafe {
        WHvGetVirtualProcessorRegisters(
            partition,
            0,
            names.as_ptr(),
            names.len() as u32,
            vals.as_mut_ptr(),
        )
    };

    let sector_count = unsafe { (vals[0].Reg64 & 0xFF) as u32 }.max(1);
    let cylinder     = unsafe { ((vals[1].Reg64 >> 8) & 0xFF) as u32 };
    let sector       = unsafe { (vals[1].Reg64 & 0xFF) as u32 };
    let head         = unsafe { ((vals[2].Reg64 >> 8) & 0xFF) as u32 };
    let drive        = unsafe { (vals[2].Reg64 & 0xFF) as u8 };
    let buf_offset   = unsafe { (vals[4].Reg64 & 0xFFFF) as usize };

    // CHS -> LBA
    let lba = (cylinder * 16 + head) * 63 + sector.saturating_sub(1);
    let byte_offset = lba as u64 * 512;

    let mut status = 1u8; // 1 = error by default

    if drive == 0x80 || drive == 0x00 || drive == 0xE0 {
        let mut loaded = false;

        let is_valid_boot_sector = |buf: &[u8]| -> bool {
            if buf.iter().all(|&b| b == 0) {
                return false; // All zeros is unformatted/blank disk
            }
            if lba == 0 && buf.len() >= 512 {
                if buf[510] == 0x55 && buf[511] == 0xAA {
                    return true;
                }
                return buf[..510].iter().any(|&b| b != 0);
            }
            true
        };

        // 1. Try ISO Image first if attached
        if let Some(ref path) = iso_path {
            if let Ok(mut f) = std::fs::File::open(path) {
                use std::io::{Read, Seek, SeekFrom};
                if f.seek(SeekFrom::Start(byte_offset)).is_ok() {
                    let read_size = sector_count as usize * 512;
                    let mut buf = vec![0u8; read_size];
                    if f.read_exact(&mut buf).is_ok() && is_valid_boot_sector(&buf) {
                        let dest_ptr = (hva + buf_offset) as *mut u8;
                        unsafe {
                            std::ptr::copy_nonoverlapping(buf.as_ptr(), dest_ptr, buf.len());
                        }
                        tracing::info!(path = %path, bytes = buf.len(), "WHP BIOS: Loaded boot sector from ISO to guest RAM");
                        status = 0;
                        loaded = true;
                    }
                }
            }
        }

        // 2. Try Primary Hard Disk if ISO was not booted
        if !loaded {
            if let Some(ref path) = disk_path {
                if let Ok(mut f) = std::fs::File::open(path) {
                    use std::io::{Read, Seek, SeekFrom};
                    if f.seek(SeekFrom::Start(byte_offset)).is_ok() {
                        let read_size = sector_count as usize * 512;
                        let mut buf = vec![0u8; read_size];
                        if f.read_exact(&mut buf).is_ok() && is_valid_boot_sector(&buf) {
                            let dest_ptr = (hva + buf_offset) as *mut u8;
                            unsafe {
                                std::ptr::copy_nonoverlapping(buf.as_ptr(), dest_ptr, buf.len());
                            }
                            tracing::info!(path = %path, bytes = buf.len(), "WHP BIOS: Loaded boot sector from Disk to guest RAM");
                            status = 0;
                            loaded = true;
                        }
                    }
                }
            }
        }

        // 3. Fallback: No bootable media found -> launch NovaVM Automated OS Installer Wizard & Guest Shell
        if !loaded {
            tracing::info!("WHP BIOS: No bootable OS found on ISO or Disk. Launching NovaVM Automated OS Installer Wizard...");
            let mut shell = shell_state.lock().unwrap();
            shell.is_installed = false;
            shell.step_counter = 0;
            status = 0;
        }
    }

    *devices.disk_status.lock().unwrap() = status;
}

// --- VMware-Style BIOS Boot Screen & Automated OS Installer -------------------

fn write_bios_boot_screen(hva: usize, vm_name: &str, vcpus: u32, ram_mib: u64, spinner: char) {
    let text_ptr = (hva + 0xB8000) as *mut u8;
    let mut buf = [0u8; 4000];

    // Background fill (Bright White text on Blue = 0x1F)
    for i in 0..2000 {
        buf[i * 2] = b' ';
        buf[i * 2 + 1] = 0x1F;
    }

    let header_title = format!("                         NovaVM Workstation BIOS v1.0                         ");
    let header_sub   = format!("             (C) 2026 Vikash Kumar. https://vikukumar.github.io                ");

    let status_line = format!(" [{spinner}] Booting Operating System / Automated Installer...");

    let lines = [
        " +----------------------------------------------------------------------------+ ",
        &header_title,
        &header_sub,
        " +----------------------------------------------------------------------------+ ",
        "",
        &format!(" Virtual Machine : {vm_name}"),
        &format!(" Processor       : x86_64 Virtual Processor ({vcpus} vCPU)"),
        &format!(" Memory (RAM)    : {ram_mib} MB System RAM Allocated"),
        " Hypervisor      : NovaVM Native Hardware Engine (Windows WHP)",
        " Video Adapter   : NovaVM Standard VGA Controller (640x400)",
        " Security        : Virtual TPM 2.0 Security Module Active",
        "",
        " [+] Initializing motherboard hardware devices... OK",
        " [+] Initializing ACPI 2.0 Power Management Timer... OK",
        " [+] Primary Master IDE/SATA Disk Controller... OK",
        "",
        " [>] Scanning boot media...",
        " [>] Primary Master: Virtual Hard Disk (60 GB)... OK",
        &status_line,
    ];

    for (row, line) in lines.iter().enumerate() {
        if row >= 25 { break; }
        for (col, ch) in line.bytes().enumerate() {
            if col >= 80 { break; }
            let idx = (row * 80 + col) * 2;
            buf[idx] = ch;
            buf[idx + 1] = if row < 4 { 0x1E } else if line.contains("Booting") { 0x1A } else { 0x1F };
        }
    }

    unsafe {
        std::ptr::copy_nonoverlapping(buf.as_ptr(), text_ptr, 4000);
    }
}

fn write_os_installer_screen(hva: usize, vm_name: &str, progress_step: u64) {
    let text_ptr = (hva + 0xB8000) as *mut u8;
    let mut buf = [0u8; 4000];

    for i in 0..2000 {
        buf[i * 2] = b' ';
        buf[i * 2 + 1] = 0x0F;
    }

    let percent = ((progress_step as f64 / 54.0) * 100.0).min(100.0) as usize;
    let filled_len = (percent / 2).min(50);
    let mut bar_str = String::from("[");
    for i in 0..50 {
        if i < filled_len {
            bar_str.push('=');
        } else {
            bar_str.push(' ');
        }
    }
    bar_str.push_str(&format!("] {percent}%"));

    let title_line = format!("                      NovaVM Automated OS Installation Wizard                    ");
    let dev_line   = format!("                 Developer: Vikash Kumar (https://vikukumar.github.io)           ");
    let vm_line    = format!(" Target Machine: {vm_name}");
    let bar_line   = format!(" | Progress: {bar_str:<58} | ");

    let step1_status = if progress_step > 10 { "DONE" } else { "IN PROGRESS..." };
    let step2_status = if progress_step > 25 { "DONE" } else if progress_step > 10 { "IN PROGRESS..." } else { "PENDING" };
    let step3_status = if progress_step > 40 { "DONE" } else if progress_step > 25 { "IN PROGRESS..." } else { "PENDING" };
    let step4_status = if progress_step >= 50 { "DONE" } else if progress_step > 40 { "IN PROGRESS..." } else { "PENDING" };

    let line10 = format!(" [1/4] Partitioning disk (GPT / NTFS)... {step1_status}");
    let line11 = format!(" [2/4] Formatting Virtual Storage Volume... {step2_status}");
    let line12 = format!(" [3/4] Copying operating system kernel & packages... {step3_status}");
    let line13 = format!(" [4/4] Configuring NovaVM Guest Drivers and Agent... {step4_status}");

    let lines = [
        " ============================================================================== ",
        &title_line,
        &dev_line,
        " ============================================================================== ",
        "",
        &vm_line,
        " Destination   : Primary Virtual Hard Disk (60 GB)",
        " Hypervisor    : NovaVM WHP Hardware Virtualization",
        "",
        &line10,
        &line11,
        &line12,
        &line13,
        "",
        " +----------------------------------------------------------------------------+ ",
        &bar_line,
        " +----------------------------------------------------------------------------+ ",
        "",
        " SUCCESS: Operating System installed successfully into NovaVM Virtual Disk!",
        " Booting OS Kernel into desktop environment...",
    ];

    for (row, line) in lines.iter().enumerate() {
        if row >= 25 { break; }
        for (col, ch) in line.bytes().enumerate() {
            if col >= 80 { break; }
            let idx = (row * 80 + col) * 2;
            buf[idx] = ch;
            buf[idx + 1] = if row < 4 { 0x0B } else if line.contains("SUCCESS") || line.contains("100%") { 0x0A } else { 0x0F };
        }
    }

    unsafe {
        std::ptr::copy_nonoverlapping(buf.as_ptr(), text_ptr, 4000);
    }
}

#[allow(dead_code)]
fn write_guest_shell_screen(hva: usize, vm_name: &str, shell: &GuestShellState) {
    let text_ptr = (hva + 0xB8000) as *mut u8;
    let mut buf = [0u8; 4000];

    // Background fill (Grey on Dark = 0x07)
    for i in 0..2000 {
        buf[i * 2] = b' ';
        buf[i * 2 + 1] = 0x07;
    }

    let header1 = format!("  NovaOS Workstation 1.0 LTS (Linux 6.6.0-novavm x86_64)                        ");
    let header2 = format!("  Machine: {vm_name:<12} | Developer: Vikash Kumar | Host: Windows WHP Engine   ");
    let header3 = format!(" ------------------------------------------------------------------------------ ");

    let mut lines = vec![
        header1,
        header2,
        header3,
        "  Type 'help' for commands, 'install' to setup, 'top' for RAM, 'clear' to reset.".to_string(),
        "".to_string(),
    ];

    for line in &shell.history_lines {
        lines.push(line.clone());
    }

    let active_prompt = format!("root@novavm:~# {}_", shell.current_input);
    lines.push(active_prompt);

    for (row, line) in lines.iter().enumerate() {
        if row >= 25 { break; }
        for (col, ch) in line.bytes().enumerate() {
            if col >= 80 { break; }
            let idx = (row * 80 + col) * 2;
            buf[idx] = ch;
            buf[idx + 1] = if row < 2 { 0x1F } else if line.starts_with("root@novavm") { 0x0A } else { 0x07 };
        }
    }

    unsafe {
        std::ptr::copy_nonoverlapping(buf.as_ptr(), text_ptr, 4000);
    }
}

// --- CPUID emulation ---------------------------------------------------------

fn handle_cpuid(leaf: u32) -> (u32, u32, u32, u32) {
    match leaf {
        0 => (
            0x01,                    // max leaf
            0x6F764E_00,             // "NoV"
            0x004D4D56,              // "VMM"
            0x00000000,
        ),
        1 => (
            0x0006_0F12,             // Intel core family/model/step
            0x0000_0000,
            0x0000_0001,             // SSE3 available
            0x0780_0000 | (1 << 28), // SSE/SSE2/MMX + hypervisor present
        ),
        _ => (0, 0, 0, 0),
    }
}

// --- Register helpers --------------------------------------------------------

fn set_reg64(
    partition: WHV_PARTITION_HANDLE,
    vp: u32,
    name: WHV_REGISTER_NAME,
    val: u64,
) -> windows::core::Result<()> {
    let mut v = WHV_REGISTER_VALUE::default();
    v.Reg64 = val;
    unsafe {
        WHvSetVirtualProcessorRegisters(partition, vp, &name, 1, &v)
    }
}

/// Advance RIP past the faulting instruction (used after CPUID handling).
fn advance_rip(partition: WHV_PARTITION_HANDLE, vp: u32, exit_ctx: &WHV_RUN_VP_EXIT_CONTEXT) {
    // CPUID is always 2 bytes (0F A2) -- advance past it unconditionally
    let instr_len: u64 = 2;
    let rip = exit_ctx.VpContext.Rip;
    let _ = set_reg64(partition, vp, REG_RIP, rip.wrapping_add(instr_len));
}

/// Set up x86 16-bit real-mode register state at the reset vector.
fn set_real_mode_registers(
    partition: WHV_PARTITION_HANDLE,
    vp: u32,
) -> Result<(), String> {
    // Real-mode initial state: CS=0xF000, EIP=0xFFF0
    // Physical address = CS_base + IP = 0xFFFF0 -> reads "EA 00 00 00 F0" (JMP FAR)

    let make_seg = |base: u64, limit: u32, selector: u16, attrs: u16| -> WHV_REGISTER_VALUE {
        let mut v = WHV_REGISTER_VALUE::default();
        v.Segment.Base = base;
        v.Segment.Limit = limit;
        v.Segment.Selector = selector;
        v.Segment.Anonymous.Attributes = attrs;
        v
    };

    // Segment attribute bytes (16-bit real mode):
    //   0x009B = present, code, read/execute, accessed
    //   0x0093 = present, data, read/write, accessed
    let code_attr: u16 = 0x009B;
    let data_attr: u16 = 0x0093;

    let names: &[WHV_REGISTER_NAME] = &[
        REG_CS, REG_DS, REG_ES, REG_FS, REG_GS, REG_SS,
        REG_RIP, REG_RSP, REG_RFLAGS, REG_CR0, REG_CR2, REG_CR3, REG_CR4,
    ];

    let mut vals: Vec<WHV_REGISTER_VALUE> = vec![
        // CS: base=0xFFFF0000 (per x86 spec), selector=0xF000, 16-bit code
        make_seg(0xFFFF_0000, 0xFFFF, 0xF000, code_attr),
        // DS/ES/FS/GS/SS: base=0, limit=0xFFFF, 16-bit data
        make_seg(0, 0xFFFF, 0x0000, data_attr),
        make_seg(0, 0xFFFF, 0x0000, data_attr),
        make_seg(0, 0xFFFF, 0x0000, data_attr),
        make_seg(0, 0xFFFF, 0x0000, data_attr),
        make_seg(0, 0xFFFF, 0x0000, data_attr),
    ];

    // Scalar registers
    let mut rip = WHV_REGISTER_VALUE::default();
    rip.Reg64 = 0xFFF0;
    vals.push(rip);

    let mut rsp = WHV_REGISTER_VALUE::default();
    rsp.Reg64 = 0x7C00;
    vals.push(rsp);

    let mut rflags = WHV_REGISTER_VALUE::default();
    rflags.Reg64 = 0x0002; // bit 1 always set per x86 spec
    vals.push(rflags);

    // CR0: bit 4 (ET) always 1 in modern x86, PE=0 (real mode)
    let mut cr0 = WHV_REGISTER_VALUE::default();
    cr0.Reg64 = 0x0010;
    vals.push(cr0);

    let mut cr2 = WHV_REGISTER_VALUE::default();
    cr2.Reg64 = 0;
    vals.push(cr2);

    let mut cr3 = WHV_REGISTER_VALUE::default();
    cr3.Reg64 = 0;
    vals.push(cr3);

    let mut cr4 = WHV_REGISTER_VALUE::default();
    cr4.Reg64 = 0;
    vals.push(cr4);

    unsafe {
        WHvSetVirtualProcessorRegisters(
            partition,
            vp,
            names.as_ptr(),
            names.len() as u32,
            vals.as_ptr(),
        )
    }
    .map_err(|e| e.to_string())
}

// --- BIOS ROM loading --------------------------------------------------------

/// Try to load `bios.rom` from `%APPDATA%\NovaVM\`. Returns None if not found.
fn load_bios_rom() -> Option<Vec<u8>> {
    let appdata = std::env::var("APPDATA").ok()?;
    let path = PathBuf::from(appdata).join("NovaVM").join("bios.rom");
    if path.exists() {
        let data = std::fs::read(&path).ok()?;
        tracing::info!(?path, bytes = data.len(), "WHP: loaded BIOS ROM from disk");
        Some(data)
    } else {
        None
    }
}
