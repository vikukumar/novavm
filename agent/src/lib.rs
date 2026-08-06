//! # NovaVM Guest Agent & Script Execution Protocol
//!
//! Defines the wire protocol for communication between the NovaVM host portal and the
//! in-guest agent (nova-agent / VMware Tools equivalent).
//!
//! Key Capabilities:
//! - In-Guest Script Execution (Bash, PowerShell, Python, CMD) with output capture
//! - Guest OS User Account Management (List, Create, Delete, Password Reset, Sync)
//! - System Metrics, Clipboard Sync, Shared Folders, USB Redirection

pub mod guest_exec;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Agent protocol version.
pub const AGENT_PROTOCOL_VERSION: u32 = 2;

/// A message sent between host and guest agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Message unique ID (used for request/response correlation).
    pub id: Uuid,
    /// Protocol version.
    pub version: u32,
    /// The message payload.
    pub payload: AgentPayload,
}

impl AgentMessage {
    pub fn new(payload: AgentPayload) -> Self {
        Self { id: Uuid::new_v4(), version: AGENT_PROTOCOL_VERSION, payload }
    }
}

/// Agent message payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum AgentPayload {
    /// Guest agent announces itself (sent on agent startup).
    Handshake(HandshakeData),
    /// Host requests guest OS information.
    GetGuestInfo,
    /// Guest responds with OS information.
    GuestInfo(GuestInfoData),
    /// Host requests script execution inside guest OS (VMware Tools Guest Exec equivalent).
    RunScript(ScriptPayload),
    /// Guest responds with script execution results.
    ScriptResult(ScriptResultData),
    /// Host requests list of OS user accounts inside the VM.
    ListUsers,
    /// Guest responds with list of OS user accounts.
    UserList(Vec<GuestUser>),
    /// Host requests creation of a new OS user account inside the VM.
    CreateUser(CreateUserData),
    /// Host requests deletion of an OS user account inside the VM.
    DeleteUser(String),
    /// Host requests password update for an OS user account inside the VM.
    UpdateUserPassword(UpdatePasswordData),
    /// Trigger bi-directional sync of portal user records and guest OS user accounts.
    SyncUsers,
    /// Host pushes clipboard content to guest.
    ClipboardSet(ClipboardData),
    /// Guest pushes clipboard content to host.
    ClipboardGet(ClipboardData),
    /// Host requests USB device list.
    GetUsbDevices,
    /// Guest responds with USB device list.
    UsbDeviceList(Vec<UsbDevice>),
    /// Host requests guest health metrics.
    GetMetrics,
    /// Guest responds with metrics.
    Metrics(GuestMetrics),
    /// Generic error response.
    Error { code: u32, message: String },
}

/// Initial handshake data sent by the guest agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandshakeData {
    pub agent_version: String,
    pub os_name: String,
    pub os_version: String,
    pub hostname: String,
}

/// Guest OS information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestInfoData {
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub hostname: String,
    pub uptime_seconds: u64,
    pub logged_in_users: Vec<String>,
}

/// Payload for running a script inside the guest OS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptPayload {
    /// Script interpreter: "powershell", "cmd", "bash", "sh", "python"
    pub interpreter: String,
    /// Full source code or command line of the script to execute
    pub script_body: String,
    /// Execution timeout in seconds (default: 60)
    pub timeout_secs: u64,
    /// Optional working directory inside guest OS
    pub working_dir: Option<String>,
    /// Environment variables to pass to the script
    pub env_vars: Option<std::collections::HashMap<String, String>>,
}

/// Output and result of script execution inside guest OS.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptResultData {
    /// Command exit code (0 = success)
    pub exit_code: i32,
    /// Standard output captured during execution
    pub stdout: String,
    /// Standard error captured during execution
    pub stderr: String,
    /// Total execution duration in milliseconds
    pub duration_ms: u64,
}

/// OS User Account representation inside the VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestUser {
    /// OS username (e.g. "Administrator", "root", "devuser")
    pub username: String,
    /// Full name or display name
    pub full_name: String,
    /// Whether user has Administrator / root privileges
    pub is_admin: bool,
    /// Whether account is disabled or locked out
    pub is_disabled: bool,
    /// ISO timestamp of last login if available
    pub last_login: Option<String>,
}

/// Parameters for creating an OS user inside the VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserData {
    pub username: String,
    pub password: String,
    pub full_name: String,
    pub is_admin: bool,
}

/// Parameters for updating an OS user's password inside the VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePasswordData {
    pub username: String,
    pub new_password: String,
}

/// Clipboard data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardData {
    pub mime_type: String,
    pub content: String,
}

/// A USB device exposed inside the guest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDevice {
    pub bus: u8,
    pub device: u8,
    pub vendor_id: u16,
    pub product_id: u16,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
    pub serial: Option<String>,
}

/// Guest-side metrics reported by the agent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuestMetrics {
    pub cpu_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub disk_read_bytes_sec: u64,
    pub disk_write_bytes_sec: u64,
}

/// Encode an agent message to a length-prefixed JSON frame.
pub fn encode_message(msg: &AgentMessage) -> Result<Vec<u8>, serde_json::Error> {
    let json = serde_json::to_vec(msg)?;
    let len = json.len() as u32;
    let mut frame = len.to_le_bytes().to_vec();
    frame.extend_from_slice(&json);
    Ok(frame)
}

/// Decode an agent message from a length-prefixed JSON frame.
pub fn decode_message(frame: &[u8]) -> Result<AgentMessage, AgentError> {
    if frame.len() < 4 {
        return Err(AgentError::InvalidFrame("Frame too short".to_owned()));
    }
    let len = u32::from_le_bytes(frame[..4].try_into().unwrap()) as usize;
    if frame.len() < 4 + len {
        return Err(AgentError::InvalidFrame("Frame truncated".to_owned()));
    }
    let msg = serde_json::from_slice(&frame[4..4 + len])
        .map_err(|e| AgentError::Serialization(e.to_string()))?;
    Ok(msg)
}

/// Agent protocol errors.
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Invalid message frame: {0}")]
    InvalidFrame(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Agent not connected to VM {0}")]
    NotConnected(Uuid),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Script execution error: {0}")]
    ExecError(String),
}
