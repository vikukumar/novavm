//! Engine error types.

use thiserror::Error;
use uuid::Uuid;

/// Errors produced by the engine and its sub-systems.
#[derive(Debug, Error)]
pub enum EngineError {
    /// A VM with the given UUID was not found in the registry.
    #[error("VM {0} not found")]
    VmNotFound(Uuid),

    /// The requested state transition is not valid from the current state.
    #[error("Invalid state transition: cannot {action} a VM in state {current_state}")]
    InvalidStateTransition { action: &'static str, current_state: String },

    /// The hypervisor backend returned an error.
    #[error("Hypervisor error: {0}")]
    Hypervisor(String),

    /// A storage operation failed.
    #[error("Storage error: {0}")]
    Storage(#[from] storage::StorageError),

    /// A network operation failed.
    #[error("Network error: {0}")]
    Network(#[from] network::NetworkError),

    /// A configuration validation error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// An I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// An unexpected internal error.
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<anyhow::Error> for EngineError {
    fn from(e: anyhow::Error) -> Self {
        Self::Internal(e.to_string())
    }
}
