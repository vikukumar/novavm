//! # NovaVM Network Subsystem
//!
//! Manages virtual switches, DHCP, DNS forwarding, and firewall integration.

pub mod switch;

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use switch::{VirtualSwitch, VirtualSwitchMode};

/// Network subsystem errors.
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("Virtual switch '{0}' not found")]
    SwitchNotFound(String),
    #[error("Switch name '{0}' already exists")]
    SwitchAlreadyExists(String),
    #[error("DHCP pool exhausted for switch '{0}'")]
    DhcpPoolExhausted(String),
    #[error("Invalid IP configuration: {0}")]
    InvalidIpConfig(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Internal network error: {0}")]
    Internal(String),
}

/// The network manager holds all virtual switches.
#[derive(Debug, Clone)]
pub struct NetworkManager {
    switches: Arc<RwLock<HashMap<String, VirtualSwitch>>>,
}

impl NetworkManager {
    /// Create a new network manager.
    pub fn new() -> Self {
        Self { switches: Arc::new(RwLock::new(HashMap::new())) }
    }

    /// Create a virtual switch.
    pub fn create_switch(
        &self,
        name: String,
        mode: VirtualSwitchMode,
    ) -> Result<Uuid, NetworkError> {
        let mut switches = self.switches.write();
        if switches.contains_key(&name) {
            return Err(NetworkError::SwitchAlreadyExists(name));
        }
        let sw = VirtualSwitch::new(name.clone(), mode);
        let id = sw.id;
        switches.insert(name, sw);
        Ok(id)
    }

    /// Create a virtual switch with full options.
    #[allow(clippy::too_many_arguments)]
    pub fn create_switch_detailed(
        &self,
        name: String,
        mode: VirtualSwitchMode,
        subnet: ipnet::Ipv4Net,
        gateway: std::net::Ipv4Addr,
        dhcp_enabled: bool,
        dhcp_range_start: std::net::Ipv4Addr,
        dhcp_range_end: std::net::Ipv4Addr,
        adapter_name: Option<String>,
    ) -> Result<Uuid, NetworkError> {
        let mut switches = self.switches.write();
        if switches.contains_key(&name) {
            return Err(NetworkError::SwitchAlreadyExists(name));
        }
        let sw = VirtualSwitch::new_detailed(
            name.clone(),
            mode,
            subnet,
            gateway,
            dhcp_enabled,
            dhcp_range_start,
            dhcp_range_end,
            adapter_name,
        );
        let id = sw.id;
        switches.insert(name, sw);
        Ok(id)
    }

    /// Update an existing virtual switch.
    #[allow(clippy::too_many_arguments)]
    pub fn update_switch(
        &self,
        name: &str,
        mode: VirtualSwitchMode,
        subnet: ipnet::Ipv4Net,
        gateway: std::net::Ipv4Addr,
        dhcp_enabled: bool,
        dhcp_range_start: std::net::Ipv4Addr,
        dhcp_range_end: std::net::Ipv4Addr,
        adapter_name: Option<String>,
    ) -> Result<(), NetworkError> {
        let mut switches = self.switches.write();
        let sw = switches
            .get_mut(name)
            .ok_or_else(|| NetworkError::SwitchNotFound(name.to_owned()))?;

        sw.mode = mode;
        sw.subnet = subnet;
        sw.gateway = gateway;
        sw.dhcp_enabled = dhcp_enabled;
        sw.dhcp_range_start = dhcp_range_start;
        sw.dhcp_range_end = dhcp_range_end;
        sw.adapter_name = adapter_name;
        Ok(())
    }

    /// Delete a virtual switch by name.
    pub fn delete_switch(&self, name: &str) -> Result<(), NetworkError> {
        self.switches
            .write()
            .remove(name)
            .ok_or_else(|| NetworkError::SwitchNotFound(name.to_owned()))
            .map(|_| ())
    }

    /// List all virtual switches.
    pub fn list_switches(&self) -> Vec<VirtualSwitch> {
        self.switches.read().values().cloned().collect()
    }

    /// Look up a switch by name.
    pub fn get_switch(&self, name: &str) -> Option<VirtualSwitch> {
        self.switches.read().get(name).cloned()
    }

    /// Enumerate real physical network adapters available on the host system.
    pub fn list_physical_adapters(&self) -> Vec<String> {
        use sysinfo::Networks;

        let networks = Networks::new_with_refreshed_list();
        let mut adapters = Vec::new();

        for (interface_name, data) in &networks {
            let mac = data.mac_address();
            let ips: Vec<String> = data.ip_networks().iter().map(|ip| ip.addr.to_string()).collect();
            let ip_str = if ips.is_empty() {
                "No IP".to_string()
            } else {
                ips.join(", ")
            };

            let entry = format!(
                "{} (IP: {}, MAC: {})",
                interface_name, ip_str, mac
            );
            adapters.push(entry);
        }

        if adapters.is_empty() {
            adapters.push("Ethernet (Default Host Interface)".to_string());
            adapters.push("Wi-Fi (Wireless LAN Adapter)".to_string());
        }

        adapters
    }
}

impl Default for NetworkManager {
    fn default() -> Self {
        Self::new()
    }
}

/// DHCP lease record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhcpLease {
    /// MAC address of the client.
    pub mac_address: String,
    /// Assigned IPv4 address.
    pub ip_address: std::net::Ipv4Addr,
    /// Lease expiry (Unix timestamp).
    pub expires_at: u64,
    /// Optional hostname.
    pub hostname: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_list_switches() {
        let mgr = NetworkManager::new();
        mgr.create_switch("nat0".to_owned(), VirtualSwitchMode::Nat).unwrap();
        mgr.create_switch("bridge0".to_owned(), VirtualSwitchMode::Bridged).unwrap();
        assert_eq!(mgr.list_switches().len(), 2);
    }

    #[test]
    fn test_duplicate_switch_name() {
        let mgr = NetworkManager::new();
        mgr.create_switch("sw0".to_owned(), VirtualSwitchMode::HostOnly).unwrap();
        assert!(mgr.create_switch("sw0".to_owned(), VirtualSwitchMode::HostOnly).is_err());
    }

    #[test]
    fn test_delete_switch() {
        let mgr = NetworkManager::new();
        mgr.create_switch("sw0".to_owned(), VirtualSwitchMode::Internal).unwrap();
        mgr.delete_switch("sw0").unwrap();
        assert!(mgr.list_switches().is_empty());
    }
}
