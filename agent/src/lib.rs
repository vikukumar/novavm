//! # NovaVM Guest Agent Protocol
//!
//! Defines the wire protocol for communication between the NovaVM host and the
//! in-guest agent (nova-agent). The guest agent runs inside the VM and provides:
//!
//! - Clipboard synchronisation
//! - Drag-and-drop file transfer
//! - Shared folder mounting
//! - USB device redirection metadata
//! - Guest OS information and health metrics
//!
//! Transport: vsock (Linux/macOS) or Hyper-V socket (Windows).
//! Framing: length-prefixed JSON messages.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Agent protocol version.
pub const AGENT_PROTOCOL_VERSION: u32 = 1;

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
    /// Create a new agent message with the given payload.
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

/// Clipboard data (text only for now; binary blobs via separate transfer).
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_roundtrip() {
        let msg = AgentMessage::new(AgentPayload::GetGuestInfo);
        let frame = encode_message(&msg).unwrap();
        let decoded = decode_message(&frame).unwrap();
        assert_eq!(decoded.id, msg.id);
        assert!(matches!(decoded.payload, AgentPayload::GetGuestInfo));
    }

    #[test]
    fn test_invalid_frame() {
        assert!(decode_message(&[0u8; 3]).is_err());
    }
}
