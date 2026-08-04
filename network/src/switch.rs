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
}

impl VirtualSwitch {
    /// Create a new virtual switch with sane defaults for the chosen mode.
    pub fn new(name: String, mode: VirtualSwitchMode) -> Self {
        // Default to 192.168.100.0/24 — callers can customise afterwards.
        let subnet: Ipv4Net = "192.168.100.0/24".parse().unwrap();
        let gateway: Ipv4Addr = "192.168.100.1".parse().unwrap();
        let dhcp_start: Ipv4Addr = "192.168.100.10".parse().unwrap();
        let dhcp_end: Ipv4Addr = "192.168.100.254".parse().unwrap();

        Self {
            id: Uuid::new_v4(),
            name,
            mode,
            subnet,
            gateway,
            dhcp_enabled: matches!(mode, VirtualSwitchMode::Nat | VirtualSwitchMode::HostOnly),
            dhcp_range_start: dhcp_start,
            dhcp_range_end: dhcp_end,
            dns_servers: vec!["8.8.8.8".parse().unwrap(), "8.8.4.4".parse().unwrap()],
            bandwidth_limit: None,
            connected_vms: 0,
            ipv6_enabled: false,
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
