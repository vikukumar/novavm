//! Virtual switch implementation.

use std::net::Ipv4Addr;

use ipnet::Ipv4Net;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Operating mode of a virtual switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VirtualSwitchMode {
    /// NAT — guests share the host's external IP via NAT; full internet access.
    Nat,
    /// Bridged — guests appear as independent hosts on the physical network.
    Bridged,
    /// Host-only — guests can talk to each other and the host, but not the internet.
    HostOnly,
    /// Internal — guests can only talk to other guests on the same switch.
    Internal,
}

impl std::fmt::Display for VirtualSwitchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VirtualSwitchMode::Nat => write!(f, "NAT"),
            VirtualSwitchMode::Bridged => write!(f, "Bridged"),
            VirtualSwitchMode::HostOnly => write!(f, "Host-only"),
            VirtualSwitchMode::Internal => write!(f, "Internal"),
        }
    }
}

/// Bandwidth limit configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthLimit {
    /// Maximum receive rate in Mbit/s (0 = unlimited).
    pub rx_mbps: u32,
    /// Maximum transmit rate in Mbit/s (0 = unlimited).
    pub tx_mbps: u32,
}

/// A virtual network switch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VirtualSwitch {
    /// Unique identifier.
    pub id: Uuid,
    /// Human-readable name (must be unique).
    pub name: String,
    /// Networking mode.
    pub mode: VirtualSwitchMode,
    /// IPv4 subnet for guest addressing.
    pub subnet: Ipv4Net,
    /// Gateway IP address (typically first host address in subnet).
    pub gateway: Ipv4Addr,
    /// Whether a built-in DHCP server is active.
    pub dhcp_enabled: bool,
    /// DHCP lease range start.
    pub dhcp_range_start: Ipv4Addr,
    /// DHCP lease range end.
    pub dhcp_range_end: Ipv4Addr,
    /// DNS servers forwarded to guests.
    pub dns_servers: Vec<std::net::IpAddr>,
    /// Optional bandwidth limit.
    pub bandwidth_limit: Option<BandwidthLimit>,
    /// Number of VMs currently connected to this switch.
    pub connected_vms: u32,
    /// IPv6 enabled.
    pub ipv6_enabled: bool,
    /// Physical adapter bound for Bridged mode.
    pub adapter_name: Option<String>,
}

impl VirtualSwitch {
    /// Create a new virtual switch with sane defaults for the chosen mode.
    pub fn new(name: String, mode: VirtualSwitchMode) -> Self {
        let (subnet_str, gw_str, start_str, end_str) = match mode {
            VirtualSwitchMode::Nat => ("192.168.128.0/24", "192.168.128.1", "192.168.128.128", "192.168.128.254"),
            VirtualSwitchMode::HostOnly => ("192.168.192.0/24", "192.168.192.1", "192.168.192.128", "192.168.192.254"),
            VirtualSwitchMode::Bridged => ("192.168.1.0/24", "192.168.1.1", "192.168.1.100", "192.168.1.200"),
            VirtualSwitchMode::Internal => ("10.0.0.0/24", "10.0.0.1", "10.0.0.10", "10.0.0.254"),
        };
        Self::new_detailed(
            name,
            mode,
            subnet_str.parse().unwrap(),
            gw_str.parse().unwrap(),
            matches!(mode, VirtualSwitchMode::Nat | VirtualSwitchMode::HostOnly),
            start_str.parse().unwrap(),
            end_str.parse().unwrap(),
            None,
        )
    }

    /// Create a virtual switch with explicit network settings.
    #[allow(clippy::too_many_arguments)]
    pub fn new_detailed(
        name: String,
        mode: VirtualSwitchMode,
        subnet: Ipv4Net,
        gateway: Ipv4Addr,
        dhcp_enabled: bool,
        dhcp_range_start: Ipv4Addr,
        dhcp_range_end: Ipv4Addr,
        adapter_name: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            mode,
            subnet,
            gateway,
            dhcp_enabled,
            dhcp_range_start,
            dhcp_range_end,
            dns_servers: vec!["8.8.8.8".parse().unwrap(), "1.1.1.1".parse().unwrap()],
            bandwidth_limit: None,
            connected_vms: 0,
            ipv6_enabled: false,
            adapter_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nat_switch_has_dhcp() {
        let sw = VirtualSwitch::new("nat0".to_owned(), VirtualSwitchMode::Nat);
        assert!(sw.dhcp_enabled);
        assert_eq!(sw.mode, VirtualSwitchMode::Nat);
    }

    #[test]
    fn test_internal_switch_no_dhcp() {
        let sw = VirtualSwitch::new("int0".to_owned(), VirtualSwitchMode::Internal);
        assert!(!sw.dhcp_enabled);
    }

    #[test]
    fn test_switch_mode_display() {
        assert_eq!(VirtualSwitchMode::Nat.to_string(), "NAT");
        assert_eq!(VirtualSwitchMode::Bridged.to_string(), "Bridged");
    }
}
