//! NovaVM Application State.
//!
//! Holds all shared state across Tauri command handlers.

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex;
use uuid::Uuid;

/// A single rendered framebuffer frame from a running VM.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FramebufferFrame {
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Base64-encoded raw RGBA bytes (width × height × 4 bytes, row-major).
    pub rgba_b64: String,
    /// Frame sequence number (monotonically increasing).
    pub seq: u64,
}

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
    /// Disk image registry.
    pub disks: Arc<parking_lot::RwLock<Vec<api::DiskMetadata>>>,
    /// Real-time application log stream.
    pub logs: Arc<parking_lot::RwLock<Vec<api::LogEntry>>>,
    /// Per-VM serial console output (UART bytes captured by native backends).
    /// Keyed by VM UUID; value is a String that accumulates UART TX output.
    pub serial_buffers: Arc<Mutex<HashMap<Uuid, String>>>,
    /// Per-VM latest rendered display framebuffer.
    /// Updated by the vCPU / framebuffer scanner thread at ~30fps.
    /// Keyed by VM UUID.
    pub framebuffers: Arc<Mutex<HashMap<Uuid, FramebufferFrame>>>,
}

impl AppState {
    /// Initialise all sub-systems.
    pub fn init() -> Self {
        tracing::info!("Initialising NovaVM application state");
        let initial_logs = vec![
            api::LogEntry {
                timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                level: "INFO".to_string(),
                target: "novavm_app".to_string(),
                message: format!("NovaVM {} starting", env!("CARGO_PKG_VERSION")),
            },
            api::LogEntry {
                timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                level: "INFO".to_string(),
                target: "engine".to_string(),
                message: "Virtualization engine initialized with native hypervisor support".to_string(),
            },
            api::LogEntry {
                timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                level: "INFO".to_string(),
                target: "network".to_string(),
                message: "Virtual network switches configured: VMnet0 (Bridged), VMnet1 (Host-Only), VMnet8 (NAT)".to_string(),
            },
            api::LogEntry {
                timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                level: "INFO".to_string(),
                target: "novavm_app".to_string(),
                message: "NovaVM application ready".to_string(),
            },
        ];

        Self {
            engine: Arc::new(Engine::new()),
            metrics: Arc::new(MetricsCollector::new()),
            network: Arc::new(NetworkManager::new()),
            settings: Arc::new(Mutex::new(AppSettings::default())),
            disks: Arc::new(parking_lot::RwLock::new(Vec::new())),
            logs: Arc::new(parking_lot::RwLock::new(initial_logs)),
            serial_buffers: Arc::new(Mutex::new(HashMap::new())),
            framebuffers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Record a real-time log entry into the in-memory application log buffer.
    pub fn push_log(&self, level: &str, target: &str, message: impl Into<String>) {
        let entry = api::LogEntry {
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            level: level.to_uppercase(),
            target: target.to_owned(),
            message: message.into(),
        };
        let mut logs = self.logs.write();
        logs.push(entry);
        if logs.len() > 1000 {
            logs.remove(0);
        }
    }

    /// Append serial output to a VM's buffer (called by vCPU threads).
    #[allow(dead_code)]
    pub fn push_serial_output(&self, vm_id: Uuid, data: &[u8]) {
        if let Ok(text) = std::str::from_utf8(data) {
            let mut bufs = self.serial_buffers.lock();
            let buf = bufs.entry(vm_id).or_default();
            buf.push_str(text);
            // Cap per-VM serial buffer at 1 MB
            if buf.len() > 1_048_576 {
                let trim = buf.len() - 1_048_576;
                buf.drain(..trim);
            }
        }
    }

    /// Drain and return all buffered serial output for a VM, clearing the buffer.
    pub fn drain_serial_output(&self, vm_id: Uuid) -> String {
        let mut bufs = self.serial_buffers.lock();
        bufs.remove(&vm_id).unwrap_or_default()
    }

    /// Store an updated framebuffer frame for a VM.
    #[allow(dead_code)]
    pub fn put_framebuffer(&self, vm_id: Uuid, frame: FramebufferFrame) {
        self.framebuffers.lock().insert(vm_id, frame);
    }

    /// Retrieve the latest framebuffer frame for a VM (if any).
    pub fn get_framebuffer(&self, vm_id: Uuid) -> Option<FramebufferFrame> {
        self.framebuffers.lock().get(&vm_id).cloned()
    }

    /// Remove the framebuffer entry for a VM (called on VM stop/destroy).
    #[allow(dead_code)]
    pub fn remove_framebuffer(&self, vm_id: Uuid) {
        self.framebuffers.lock().remove(&vm_id);
    }

    /// Persist all registered VMs to disk JSON storage.
    pub async fn sync_vms_to_disk(&self) {
        let storage_dir = self.settings.lock().default_storage_dir.clone();
        let persistence = crate::persistence::Persistence::new(storage_dir);
        let mut configs = Vec::new();
        for id in self.engine.registry().ids() {
            if let Some(handle) = self.engine.registry().get(&id) {
                let vm = handle.read().await;
                configs.push(vm.config().clone());
            }
        }
        persistence.save_vms(&configs);
    }

    /// Persist all managed disk metadata to disk JSON storage.
    pub fn sync_disks_to_disk(&self) {
        let storage_dir = self.settings.lock().default_storage_dir.clone();
        let persistence = crate::persistence::Persistence::new(storage_dir);
        persistence.save_disks(&self.disks.read());
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
