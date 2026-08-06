//! NovaVM Tauri Application Entry Point.
//!
//! Initialises all sub-systems, sets up the tokio async runtime, registers
//! all Tauri IPC command handlers, and starts the desktop application window.

// Prevent Windows console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod persistence;
mod state;

use std::time::Duration;

use tauri::Manager;

use monitor::MetricsCollector;
use state::AppState;

fn main() {
    tracing::info!("NovaVM {} starting", env!("CARGO_PKG_VERSION"));

    // Build and run the Tauri application.
    tauri::Builder::default()
        // ── Plugins ───────────────────────────────────────────────────────────
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_log::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::default().build())
        // ── Application State ─────────────────────────────────────────────────
        .manage(AppState::init())
        // ── Setup Hook ───────────────────────────────────────────────────────
        .setup(|app| {
            let state: tauri::State<AppState> = app.state();
            // Spawn background metrics collection.
            let metrics_owned: MetricsCollector = (*state.metrics).clone();
            tauri::async_runtime::spawn(async move {
                metrics_owned.run_background_collection(Duration::from_secs(1)).await;
            });

            // Initialise default VMware Workstation style virtual networks.
            if state.network.get_switch("VMnet8 (NAT)").is_none() {
                state
                    .network
                    .create_switch("VMnet8 (NAT)".to_owned(), network::VirtualSwitchMode::Nat)
                    .ok();
            }
            if state.network.get_switch("VMnet1 (Host-Only)").is_none() {
                state
                    .network
                    .create_switch("VMnet1 (Host-Only)".to_owned(), network::VirtualSwitchMode::HostOnly)
                    .ok();
            }
            if state.network.get_switch("VMnet0 (Bridged)").is_none() {
                state
                    .network
                    .create_switch("VMnet0 (Bridged)".to_owned(), network::VirtualSwitchMode::Bridged)
                    .ok();
            }
            tracing::info!("VMware-style virtual networks initialized");

            // Restore persisted VMs and Disks from storage directory
            let storage_dir = state.settings.lock().default_storage_dir.clone();
            let persistence = persistence::Persistence::new(&storage_dir);
            let saved_vms = persistence.load_vms();
            let saved_disks = persistence.load_disks();

            *state.disks.write() = saved_disks;

            let engine = state.engine.clone();
            let metrics = state.metrics.clone();
            tauri::async_runtime::spawn(async move {
                for cfg in saved_vms {
                    let name = cfg.name.clone();
                    if let Ok(id) = engine.create_vm(cfg).await {
                        metrics.register_vm(id);
                        tracing::info!(%id, name = %name, "Restored persisted VM on startup");
                    }
                }
            });

            tracing::info!("NovaVM application ready");
            Ok(())
        })
        // ── IPC Commands ──────────────────────────────────────────────────────
        .invoke_handler(tauri::generate_handler![
            // VM commands
            commands::vm::list_vms,
            commands::vm::create_vm,
            commands::vm::get_vm,
            commands::vm::start_vm,
            commands::vm::pause_vm,
            commands::vm::resume_vm,
            commands::vm::stop_vm,
            commands::vm::reset_vm,
            commands::vm::destroy_vm,
            commands::vm::update_vm_config,
            commands::vm::open_vm_display,
            commands::vm::get_vm_serial_output,
            commands::vm::run_guest_script,
            commands::vm::list_guest_users,
            commands::vm::create_guest_user,
            commands::vm::update_guest_user_password,
            commands::vm::sync_guest_users,
            // Monitor commands
            commands::monitor::get_host_metrics,
            commands::monitor::get_vm_metrics,
            commands::monitor::get_host_metrics_history,
            commands::monitor::get_vm_metrics_history,
            commands::monitor::get_application_logs,
            commands::monitor::clear_application_logs,
            // Network commands
            commands::network::list_switches,
            commands::network::create_switch,
            commands::network::update_switch,
            commands::network::delete_switch,
            commands::network::list_physical_adapters,
            // Storage commands
            commands::storage::list_disks,
            commands::storage::create_disk,
            commands::storage::import_disk,
            commands::storage::delete_disk,
            // Snapshot commands
            commands::snapshot::take_snapshot,
            // Settings commands
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::get_app_version,
            commands::settings::get_hypervisor_info,
            commands::settings::get_qemu_status,
            commands::settings::get_virtualization_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running NovaVM application");
}
