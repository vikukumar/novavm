//! NovaVM Application State.
//!
//! Holds all shared state across Tauri command handlers.

use std::sync::Arc;

use parking_lot::Mutex;

use engine::Engine;
use monitor::MetricsCollector;
use network::NetworkManager;

/// Global application state managed by Tauri.
pub struct AppState {
    /// The VM lifecycle engine.
    pub engine: Arc<Engine>,
    /// Real-time metrics collector.
    pub metrics: Arc<MetricsCollector>,
    /// Virtual network manager.
    pub network: Arc<NetworkManager>,
    /// Application-level settings.
    pub settings: Arc<Mutex<AppSettings>>,
}

impl AppState {
    /// Initialise all sub-systems.
    pub fn init() -> Self {
        tracing::info!("Initialising NovaVM application state");
        Self {
            engine: Arc::new(Engine::new()),
            metrics: Arc::new(MetricsCollector::new()),
            network: Arc::new(NetworkManager::new()),
            settings: Arc::new(Mutex::new(AppSettings::default())),
        }
    }
}

/// User-editable application settings.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    /// UI color theme.
    pub theme: Theme,
    /// Default VM storage directory.
    pub default_storage_dir: String,
    /// Default ISO library directory.
    pub default_iso_dir: String,
    /// Whether to start the background service automatically.
    pub auto_start_service: bool,
    /// Metrics collection interval in seconds.
    pub metrics_interval_secs: u64,
    /// Telemetry / crash reporting opt-in.
    pub telemetry_enabled: bool,
    /// Application language code (e.g. "en-US").
    pub language: String,
}

/// UI color theme.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Light,
    Dark,
    System,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            default_storage_dir: default_storage_dir(),
            default_iso_dir: default_iso_dir(),
            auto_start_service: true,
            metrics_interval_secs: 1,
            telemetry_enabled: false,
            language: "en-US".to_owned(),
        }
    }
}

fn default_storage_dir() -> String {
    dirs_next::data_dir()
        .map(|d| d.join("NovaVM").join("vms").to_string_lossy().to_string())
        .unwrap_or_else(|| "./vms".to_owned())
}

fn default_iso_dir() -> String {
    dirs_next::data_dir()
        .map(|d| d.join("NovaVM").join("iso").to_string_lossy().to_string())
        .unwrap_or_else(|| "./iso".to_owned())
}
