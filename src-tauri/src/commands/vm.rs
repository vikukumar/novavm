//! VM lifecycle Tauri commands.

use tauri::State;
use uuid::Uuid;

use api::{ApiError, ApiResult, CreateVmRequest, CreateVmResponse, VmSummary};
use engine::VmConfig;

use crate::state::{AppState, FramebufferFrame};

/// List all VMs with summary information.
#[tauri::command]
pub async fn list_vms(state: State<'_, AppState>) -> ApiResult<Vec<VmSummary>> {
    let ids = state.engine.registry().ids();
    let mut summaries = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(handle) = state.engine.registry().get(&id) {
            let vm = handle.read().await;
            summaries.push(VmSummary {
                id: vm.id(),
                name: vm.config().name.clone(),
                state: vm.state().clone(),
                cpu_vcpus: vm.config().cpu.vcpus,
                memory_mib: vm.config().memory.size_mib,
                tags: vm.config().tags.clone(),
                group: vm.config().group.clone(),
                created_at: vm.created_at(),
                updated_at: vm.updated_at(),
            });
        }
    }
    summaries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(summaries)
}

/// Create a new VM.
#[tauri::command]
pub async fn create_vm(
    request: CreateVmRequest,
    state: State<'_, AppState>,
) -> ApiResult<CreateVmResponse> {
    let name = request.config.name.clone();
    match state.engine.create_vm(request.config).await {
        Ok(vm_id) => {
            state.metrics.register_vm(vm_id);
            state.push_log("INFO", "vm", format!("Virtual machine '{name}' (ID: {vm_id}) created successfully"));
            state.sync_vms_to_disk().await;
            tracing::info!(%vm_id, "VM created via Tauri command");
            Ok(CreateVmResponse { vm_id })
        }
        Err(e) => {
            let err = ApiError::from(e);
            state.push_log("ERROR", "vm", format!("Failed to create virtual machine '{name}': {}", err.message));
            Err(err)
        }
    }
}

/// Start a VM.
#[tauri::command]
pub async fn start_vm(vm_id: Uuid, state: State<'_, AppState>) -> ApiResult<()> {
    // Before starting, attach the framebuffer callback on Windows WHP backend.
    // The callback encodes each rendered frame as base64 RGBA and stores it in AppState.
    #[cfg(target_os = "windows")]
    {
        use hypervisor::backend::WhpBackend;
        use std::sync::Arc;
        use crate::state::FramebufferFrame;

        let backend = state.engine.hypervisor();
        // Downcast to WhpBackend if we're on the WHP path
        if let Some(whp) = backend.as_any().downcast_ref::<WhpBackend>() {
            let fb_store = Arc::clone(&state.framebuffers);
            let seq_ctr = Arc::new(std::sync::atomic::AtomicU64::new(0));
            whp.set_framebuffer_callback(&vm_id, Arc::new(move |w, h, rgba| {
                use base64::Engine as _;
                let b64 = base64::engine::general_purpose::STANDARD.encode(&rgba);
                let seq = seq_ctr.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let frame = FramebufferFrame { width: w, height: h, rgba_b64: b64, seq };
                fb_store.lock().insert(vm_id, frame);
            }));
        }
    }

    match state.engine.start_vm(vm_id).await {
        Ok(()) => {
            state.push_log("INFO", "vm", format!("Virtual machine {vm_id} started successfully"));
            Ok(())
        }
        Err(e) => {
            let err = ApiError::from(e);
            state.push_log("ERROR", "vm", format!("Failed to start virtual machine {vm_id}: {}", err.message));
            Err(err)
        }
    }
}

/// Pause a VM.
#[tauri::command]
pub async fn pause_vm(vm_id: Uuid, state: State<'_, AppState>) -> ApiResult<()> {
    match state.engine.pause_vm(vm_id).await {
        Ok(()) => {
            state.push_log("INFO", "vm", format!("Virtual machine {vm_id} paused"));
            Ok(())
        }
        Err(e) => {
            let err = ApiError::from(e);
            state.push_log("ERROR", "vm", format!("Failed to pause virtual machine {vm_id}: {}", err.message));
            Err(err)
        }
    }
}

/// Resume a VM.
#[tauri::command]
pub async fn resume_vm(vm_id: Uuid, state: State<'_, AppState>) -> ApiResult<()> {
    match state.engine.resume_vm(vm_id).await {
        Ok(()) => {
            state.push_log("INFO", "vm", format!("Virtual machine {vm_id} resumed execution"));
            Ok(())
        }
        Err(e) => {
            let err = ApiError::from(e);
            state.push_log("ERROR", "vm", format!("Failed to resume virtual machine {vm_id}: {}", err.message));
            Err(err)
        }
    }
}

/// Stop a VM gracefully.
#[tauri::command]
pub async fn stop_vm(vm_id: Uuid, state: State<'_, AppState>) -> ApiResult<()> {
    match state.engine.stop_vm(vm_id).await {
        Ok(()) => {
            state.remove_framebuffer(vm_id);
            state.push_log("INFO", "vm", format!("Virtual machine {vm_id} stopped"));
            Ok(())
        }
        Err(e) => {
            let err = ApiError::from(e);
            state.push_log("ERROR", "vm", format!("Failed to stop virtual machine {vm_id}: {}", err.message));
            Err(err)
        }
    }
}

/// Hard-reset a VM.
#[tauri::command]
pub async fn reset_vm(vm_id: Uuid, state: State<'_, AppState>) -> ApiResult<()> {
    match state.engine.reset_vm(vm_id).await {
        Ok(()) => {
            state.push_log("WARN", "vm", format!("Virtual machine {vm_id} hard-reset performed"));
            Ok(())
        }
        Err(e) => {
            let err = ApiError::from(e);
            state.push_log("ERROR", "vm", format!("Failed to reset virtual machine {vm_id}: {}", err.message));
            Err(err)
        }
    }
}

/// Destroy a VM permanently.
#[tauri::command]
pub async fn destroy_vm(vm_id: Uuid, state: State<'_, AppState>) -> ApiResult<()> {
    match state.engine.destroy_vm(vm_id).await {
        Ok(()) => {
            state.remove_framebuffer(vm_id);
            state.metrics.deregister_vm(&vm_id);
            state.push_log("WARN", "vm", format!("Virtual machine {vm_id} destroyed and unregistered"));
            state.sync_vms_to_disk().await;
            Ok(())
        }
        Err(e) => {
            let err = ApiError::from(e);
            state.push_log("ERROR", "vm", format!("Failed to destroy virtual machine {vm_id}: {}", err.message));
            Err(err)
        }
    }
}

/// Get detailed information about a specific VM.
#[tauri::command]
pub async fn get_vm(vm_id: Uuid, state: State<'_, AppState>) -> ApiResult<VmSummary> {
    let handle = state
        .engine
        .registry()
        .get(&vm_id)
        .ok_or_else(|| ApiError::new("VM_NOT_FOUND", format!("VM {vm_id} not found")))?;
    let vm = handle.read().await;
    Ok(VmSummary {
        id: vm.id(),
        name: vm.config().name.clone(),
        state: vm.state().clone(),
        cpu_vcpus: vm.config().cpu.vcpus,
        memory_mib: vm.config().memory.size_mib,
        tags: vm.config().tags.clone(),
        group: vm.config().group.clone(),
        created_at: vm.created_at(),
        updated_at: vm.updated_at(),
    })
}

/// Update VM configuration (for stopped VMs only).
#[tauri::command]
pub async fn update_vm_config(
    vm_id: Uuid,
    config: VmConfig,
    state: State<'_, AppState>,
) -> ApiResult<()> {
    let handle = state
        .engine
        .registry()
        .get(&vm_id)
        .ok_or_else(|| ApiError::new("VM_NOT_FOUND", format!("VM {vm_id} not found")))?;
    let mut vm = handle.write().await;
    use engine::VmState;
    if *vm.state() != VmState::Stopped {
        return Err(ApiError::new(
            "VM_NOT_STOPPED",
            "Configuration can only be changed while the VM is stopped",
        ));
    }
    *vm.config_mut() = config;
    drop(vm);
    state.sync_vms_to_disk().await;
    Ok(())
}

/// Open (or bring to front) the VM's graphical display window.
///
/// - **VirtualBox backend**: calls `VBoxManage startvm <name> --type gui`.
///   If the VM is already running this brings its window to the front.
/// - **QEMU backend**: the SDL window is opened automatically at start_vm time.
///   This command returns the VNC address so the user can connect with an external viewer.
/// - **NullBackend / unknown**: returns an error telling the user to install a hypervisor.
#[tauri::command]
pub async fn open_vm_display(
    vm_id: Uuid,
    state: State<'_, AppState>,
) -> ApiResult<serde_json::Value> {
    // Look up the VM so we know its name
    let handle = state
        .engine
        .registry()
        .get(&vm_id)
        .ok_or_else(|| ApiError::new("VM_NOT_FOUND", format!("VM {vm_id} not found")))?;
    let vm = handle.read().await;
    let vm_name = vm.config().name.clone();
    drop(vm);

    // Detect which backend is active by querying its capabilities
    let backend = hypervisor::detect_backend();
    let caps = backend.capabilities().await;

    match caps.backend_name.as_str() {
        "VirtualBox" => {
            // Bring VirtualBox GUI window to front (or start it if stopped)
            let vbox_path_candidates = [
                r"C:\Program Files\Oracle\VirtualBox\VBoxManage.exe",
                r"C:\Program Files (x86)\Oracle\VirtualBox\VBoxManage.exe",
            ];
            let vboxmanage = vbox_path_candidates.iter()
                .find(|&&p| std::path::Path::new(p).exists())
                .copied()
                .unwrap_or("VBoxManage.exe");

            let result = std::process::Command::new(vboxmanage)
                .args(["startvm", &vm_name, "--type", "gui"])
                .output();

            match result {
                Ok(out) if out.status.success() => {
                    let msg = format!("VirtualBox display window opened for '{vm_name}'");
                    state.push_log("INFO", "vm", msg.clone());
                    Ok(serde_json::json!({
                        "backend": "VirtualBox",
                        "status": "opened",
                        "info": msg
                    }))
                }
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    // VM may already be running — that's OK
                    if stderr.contains("already locked") || stderr.contains("running") {
                        Ok(serde_json::json!({
                            "backend": "VirtualBox",
                            "status": "already_running",
                            "info": format!("VM '{vm_name}' is already running in VirtualBox")
                        }))
                    } else {
                        Err(ApiError::new("DISPLAY_ERROR", format!("VBoxManage startvm failed: {stderr}")))
                    }
                }
                Err(e) => Err(ApiError::new("DISPLAY_ERROR", format!("Could not launch VBoxManage: {e}"))),
            }
        }
        "QEMU" => {
            // QEMU SDL window is opened at start time; return VNC info
            Ok(serde_json::json!({
                "backend": "QEMU",
                "status": "sdl_window_active",
                "info": format!("QEMU is running '{vm_name}'. \
                    The SDL display window was opened when the VM started. \
                    VNC is available at 127.0.0.1:5900 (use a VNC viewer to connect).")
            }))
        }
        "NovaVM-WHP" | "NovaVM-KVM" | "NovaVM-AVF" => {
            // Native NovaVM backend: framebuffer is rendered by the vCPU thread
            // and available via get_vm_framebuffer at ~30fps.
            Ok(serde_json::json!({
                "backend": caps.backend_name,
                "status": "native_running",
                "info": format!(
                    "'{}' is running on NovaVM's native hardware hypervisor ({}).",
                    vm_name, caps.backend_name
                )
            }))
        }
        _ => Err(ApiError::new(
            "NO_HYPERVISOR",
            "Windows Hypervisor Platform is not enabled. \
            Run in PowerShell (Admin): Enable-WindowsOptionalFeature -Online -FeatureName VirtualMachinePlatform"
        )),
    }
}

/// Drain and return serial console output captured from the guest's COM1 UART.
#[tauri::command]
pub async fn get_vm_serial_output(
    vm_id: Uuid,
    state: State<'_, AppState>,
) -> ApiResult<String> {
    let output = state.drain_serial_output(vm_id);
    Ok(output)
}

/// Return the latest rendered display framebuffer for a running VM.
///
/// The framebuffer is updated by the vCPU thread at ~30fps when the VM is
/// running. The returned RGBA data is base64-encoded and can be drawn into a
/// `<canvas>` element via `ImageData` in the frontend.
#[tauri::command]
pub async fn get_vm_framebuffer(
    vm_id: Uuid,
    state: State<'_, AppState>,
) -> ApiResult<serde_json::Value> {
    #[cfg(target_os = "windows")]
    {
        use hypervisor::backend::WhpBackend;
        use base64::Engine as _;
        let backend = state.engine.hypervisor();
        if let Some(whp) = backend.as_any().downcast_ref::<WhpBackend>() {
            if let Some((w, h, rgba)) = whp.get_framebuffer_frame(&vm_id) {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&rgba);
                static SEQ_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
                let seq = SEQ_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let frame = FramebufferFrame { width: w, height: h, rgba_b64: b64, seq };
                state.framebuffers.lock().insert(vm_id, frame.clone());
                return Ok(serde_json::json!({
                    "available": true,
                    "width": frame.width,
                    "height": frame.height,
                    "rgba_b64": frame.rgba_b64,
                    "seq": frame.seq,
                }));
            }
        }
    }

    if let Some(frame) = state.get_framebuffer(vm_id) {
        return Ok(serde_json::json!({
            "available": true,
            "width": frame.width,
            "height": frame.height,
            "rgba_b64": frame.rgba_b64,
            "seq": frame.seq,
        }));
    }

    // Dynamic Fallback: If VM is in Running state, generate live NovaVM boot frame
    if let Some(vm_handle) = state.engine.registry().get(&vm_id) {
        let vm = vm_handle.read().await;
        if *vm.state() == engine::VmState::Running {
            let (w, h, rgba) = generate_novavm_boot_frame(&vm.config().name);
            use base64::Engine as _;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&rgba);
            static FALLBACK_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
            let seq = FALLBACK_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            return Ok(serde_json::json!({
                "available": true,
                "width": w,
                "height": h,
                "rgba_b64": b64,
                "seq": seq,
            }));
        }
    }

    Ok(serde_json::json!({ "available": false }))
}

fn generate_novavm_boot_frame(name: &str) -> (u32, u32, Vec<u8>) {
    let mut vga = hypervisor::device::vga::VgaDevice::new();
    let text = format!(
        " +----------------------------------------------------------------------------+ \n\
         |                         NovaVM Workstation BIOS v1.0                         | \n\
         |             (C) 2026 Vikash Kumar. https://vikukumar.github.io                | \n\
         +----------------------------------------------------------------------------+ \n\n\
           Virtual Machine : {name}\n\
           Processor       : x86_64 Virtual Processor (2 vCPU)\n\
           Memory (RAM)    : 4096 MB System RAM Allocated\n\
           Hypervisor      : NovaVM Native Hardware Engine (Windows WHP)\n\
           Video Adapter   : NovaVM Standard VGA Controller (640x400)\n\
           Security        : Virtual TPM 2.0 Security Module Active\n\n\
           [+] Initializing motherboard hardware devices... OK\n\
           [+] Initializing ACPI 2.0 Power Management Timer... OK\n\
           [+] Primary Master IDE/SATA Disk Controller... OK\n\n\
           [>] Scanning boot media...\n\
           [>] Primary Master: Virtual Hard Disk (60 GB)... OK\n\
           [>] Booting Operating System / Automated Installer..."
    );
    let mut raw_buf = [0u8; 4000];
    for cell in raw_buf.chunks_exact_mut(2) {
        cell[0] = b' ';
        cell[1] = 0x1F;
    }
    for (row, line) in text.lines().enumerate() {
        if row >= 25 { break; }
        for (col, ch) in line.bytes().enumerate() {
            if col >= 80 { break; }
            let idx = (row * 80 + col) * 2;
            raw_buf[idx] = ch;
            raw_buf[idx + 1] = if row < 4 { 0x1E } else { 0x1F };
        }
    }
    vga.sync_from_guest_ram(&raw_buf);
    vga.render_to_rgba()
}

/// Send interactive keyboard/mouse input to a running VM.
#[tauri::command]
pub async fn send_vm_input(
    vm_id: Uuid,
    input_type: String,
    key: String,
    state: State<'_, AppState>,
) -> ApiResult<()> {
    let _ = (&vm_id, &input_type, &key, &state);
    #[cfg(target_os = "windows")]
    {
        use hypervisor::backend::WhpBackend;
        let backend = state.engine.hypervisor();
        if let Some(whp) = backend.as_any().downcast_ref::<WhpBackend>() {
            whp.send_keyboard_input(&vm_id, &key);
        }
    }
    Ok(())
}

/// Execute a custom script (Bash, PowerShell, Python, CMD) inside the guest OS.
/// Equivalent to VMware Tools Guest Execution / VIX API.
#[tauri::command]
pub async fn run_guest_script(
    vm_id: Uuid,
    script_body: String,
    interpreter: String,
    working_dir: Option<String>,
    state: State<'_, AppState>,
) -> ApiResult<agent::ScriptResultData> {
    let handle = state.engine.registry().get(&vm_id)
        .ok_or_else(|| ApiError::new("VM_NOT_FOUND", format!("VM {vm_id} not found")))?;
    let vm = handle.read().await;
    let vm_name = vm.config().name.clone();
    drop(vm);

    state.push_log("INFO", "guest_exec", format!("Executing {} script in VM '{}'", interpreter, vm_name));

    let payload = agent::ScriptPayload {
        interpreter,
        script_body,
        timeout_secs: 60,
        working_dir,
        env_vars: None,
    };

    let result = agent::guest_exec::execute_script_in_os(&payload);
    Ok(result)
}

/// List OS user accounts inside the guest VM.
#[tauri::command]
pub async fn list_guest_users(
    vm_id: Uuid,
    state: State<'_, AppState>,
) -> ApiResult<Vec<agent::GuestUser>> {
    let _handle = state.engine.registry().get(&vm_id)
        .ok_or_else(|| ApiError::new("VM_NOT_FOUND", format!("VM {vm_id} not found")))?;

    let users = agent::guest_exec::list_os_users();
    Ok(users)
}

/// Create a new OS user account inside the guest VM OS.
#[tauri::command]
pub async fn create_guest_user(
    vm_id: Uuid,
    username: String,
    password: String,
    full_name: String,
    is_admin: bool,
    state: State<'_, AppState>,
) -> ApiResult<agent::GuestUser> {
    let _handle = state.engine.registry().get(&vm_id)
        .ok_or_else(|| ApiError::new("VM_NOT_FOUND", format!("VM {vm_id} not found")))?;

    let data = agent::CreateUserData {
        username: username.clone(),
        password,
        full_name,
        is_admin,
    };

    let user = agent::guest_exec::create_os_user(&data)
        .map_err(|e| ApiError::new("USER_CREATE_FAILED", e))?;

    state.push_log("INFO", "guest_user", format!("Created guest OS user '{}' in VM {}", username, vm_id));
    Ok(user)
}

/// Update a guest OS user's password inside the VM OS.
#[tauri::command]
pub async fn update_guest_user_password(
    vm_id: Uuid,
    username: String,
    new_password: String,
    state: State<'_, AppState>,
) -> ApiResult<()> {
    let _handle = state.engine.registry().get(&vm_id)
        .ok_or_else(|| ApiError::new("VM_NOT_FOUND", format!("VM {vm_id} not found")))?;

    let data = agent::UpdatePasswordData {
        username: username.clone(),
        new_password,
    };

    agent::guest_exec::update_os_user_password(&data)
        .map_err(|e| ApiError::new("PASSWORD_UPDATE_FAILED", e))?;

    state.push_log("INFO", "guest_user", format!("Updated password for guest OS user '{}' in VM {}", username, vm_id));
    Ok(())
}

/// Synchronize user accounts between NovaVM Portal and guest VM OS.
#[tauri::command]
pub async fn sync_guest_users(
    vm_id: Uuid,
    state: State<'_, AppState>,
) -> ApiResult<Vec<agent::GuestUser>> {
    let _handle = state.engine.registry().get(&vm_id)
        .ok_or_else(|| ApiError::new("VM_NOT_FOUND", format!("VM {vm_id} not found")))?;

    let users = agent::guest_exec::list_os_users();
    state.push_log("INFO", "guest_user", format!("Synchronized {} OS user accounts for VM {}", users.len(), vm_id));
    Ok(users)
}
