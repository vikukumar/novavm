//! NovaVM Tauri Application Entry Point.
//!
//! Initialises all sub-systems, sets up the tokio async runtime, registers
//! all Tauri IPC command handlers, and starts the desktop application window.

// Prevent Windows console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

use std::time::Duration;

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use tauri::Manager;

use monitor::MetricsCollector;
use state::AppState;

fn main() {
    // Initialise structured logging.
    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true).with_thread_ids(true))
        .with(EnvFilter::from_default_env().add_directive("novavm=debug".parse().unwrap()))
        .init();

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
            // start_background_collection takes `self: MetricsCollector` by value.
            // MetricsCollector is Clone (backed by Arcs), so cloning is cheap.
            let metrics_owned: MetricsCollector = (*state.metrics).clone();
            let _ = metrics_owned.start_background_collection(Duration::from_secs(1));

            // Initialise default NAT switch.
            if state.network.get_switch("default-nat").is_none() {
                state
                    .network
                    .create_switch(
                        "default-nat".to_owned(),
                        network::VirtualSwitchMode::Nat,
                    )
                    .ok();
                tracing::info!("Default NAT switch created");
            }

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
            // Monitor commands
            commands::monitor::get_host_metrics,
            commands::monitor::get_vm_metrics,
            commands::monitor::get_host_metrics_history,
            commands::monitor::get_vm_metrics_history,
            // Network commands
            commands::network::list_switches,
            commands::network::create_switch,
            commands::network::delete_switch,
            // Storage commands
            commands::storage::list_disks,
            commands::storage::create_disk,
            // Snapshot commands
            commands::snapshot::take_snapshot,
            // Settings commands
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::get_app_version,
            commands::settings::get_hypervisor_info,
        ])
        .run(tauri::generate_context!())
        .expect("error while running NovaVM application");
}
