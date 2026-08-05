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
    let vm_id = state.engine.create_vm(request.config).await.map_err(ApiError::from)?;
    state.metrics.register_vm(vm_id);
    state.push_log("INFO", "vm", format!("Virtual machine '{name}' (ID: {vm_id}) created successfully"));
    state.sync_vms_to_disk().await;
    tracing::info!(%vm_id, "VM created via Tauri command");
    Ok(CreateVmResponse { vm_id })
}

/// Start a VM.
#[tauri::command]
pub async fn start_vm(vm_id: Uuid, state: State<'_, AppState>) -> ApiResult<()> {
    state.engine.start_vm(vm_id).await.map_err(ApiError::from)?;
    state.push_log("INFO", "vm", format!("Virtual machine {vm_id} started"));
    Ok(())
}

/// Pause a VM.
#[tauri::command]
pub async fn pause_vm(vm_id: Uuid, state: State<'_, AppState>) -> ApiResult<()> {
    state.engine.pause_vm(vm_id).await.map_err(ApiError::from)?;
    state.push_log("INFO", "vm", format!("Virtual machine {vm_id} paused"));
    Ok(())
}

/// Resume a VM.
#[tauri::command]
pub async fn resume_vm(vm_id: Uuid, state: State<'_, AppState>) -> ApiResult<()> {
    state.engine.resume_vm(vm_id).await.map_err(ApiError::from)?;
    state.push_log("INFO", "vm", format!("Virtual machine {vm_id} resumed execution"));
    Ok(())
}

/// Stop a VM gracefully.
#[tauri::command]
pub async fn stop_vm(vm_id: Uuid, state: State<'_, AppState>) -> ApiResult<()> {
    state.engine.stop_vm(vm_id).await.map_err(ApiError::from)?;
    state.push_log("INFO", "vm", format!("Virtual machine {vm_id} stopped"));
    Ok(())
}

/// Hard-reset a VM.
#[tauri::command]
pub async fn reset_vm(vm_id: Uuid, state: State<'_, AppState>) -> ApiResult<()> {
    state.engine.reset_vm(vm_id).await.map_err(ApiError::from)?;
    state.push_log("WARN", "vm", format!("Virtual machine {vm_id} hard-reset performed"));
    Ok(())
}

/// Destroy a VM permanently.
#[tauri::command]
pub async fn destroy_vm(vm_id: Uuid, state: State<'_, AppState>) -> ApiResult<()> {
    state.engine.destroy_vm(vm_id).await.map_err(ApiError::from)?;
    state.metrics.deregister_vm(&vm_id);
    state.push_log("WARN", "vm", format!("Virtual machine {vm_id} destroyed and unregistered"));
    state.sync_vms_to_disk().await;
    Ok(())
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
