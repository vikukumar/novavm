//! VM lifecycle Tauri commands.

use tauri::State;
use uuid::Uuid;

use api::{ApiError, ApiResult, CreateVmRequest, CreateVmResponse, VmSummary};
use engine::VmConfig;

use crate::state::AppState;

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
        _ => Err(ApiError::new(
            "NO_HYPERVISOR",
            "No hypervisor is installed. Install VirtualBox or QEMU to run virtual machines."
        )),
    }
}

