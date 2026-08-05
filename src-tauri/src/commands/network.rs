//! Network Tauri commands.

use std::net::Ipv4Addr;
use ipnet::Ipv4Net;
use tauri::State;

use api::{ApiError, ApiResult, VirtualSwitch, VirtualSwitchMode};

use crate::state::AppState;

/// List all virtual switches.
#[tauri::command]
pub async fn list_switches(state: State<'_, AppState>) -> ApiResult<Vec<VirtualSwitch>> {
    Ok(state.network.list_switches())
}

/// Create a virtual switch with detailed options.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn create_switch(
    name: String,
    mode: VirtualSwitchMode,
    subnet: Option<String>,
    gateway: Option<String>,
    dhcp_enabled: Option<bool>,
    dhcp_range_start: Option<String>,
    dhcp_range_end: Option<String>,
    adapter_name: Option<String>,
    state: State<'_, AppState>,
) -> ApiResult<String> {
    let parsed_subnet: Ipv4Net = match subnet {
        Some(s) if !s.trim().is_empty() => s.parse().map_err(|e| ApiError::new("INVALID_SUBNET", format!("Invalid subnet CIDR: {e}")))?,
        _ => match mode {
            VirtualSwitchMode::Nat => "192.168.128.0/24".parse().unwrap(),
            VirtualSwitchMode::HostOnly => "192.168.192.0/24".parse().unwrap(),
            VirtualSwitchMode::Bridged => "192.168.1.0/24".parse().unwrap(),
            VirtualSwitchMode::Internal => "10.0.0.0/24".parse().unwrap(),
        },
    };

    let parsed_gw: Ipv4Addr = match gateway {
        Some(g) if !g.trim().is_empty() => g.parse().map_err(|e| ApiError::new("INVALID_GATEWAY", format!("Invalid gateway IP: {e}")))?,
        _ => match mode {
            VirtualSwitchMode::Nat => "192.168.128.1".parse().unwrap(),
            VirtualSwitchMode::HostOnly => "192.168.192.1".parse().unwrap(),
            VirtualSwitchMode::Bridged => "192.168.1.1".parse().unwrap(),
            VirtualSwitchMode::Internal => "10.0.0.1".parse().unwrap(),
        },
    };

    let is_dhcp = dhcp_enabled.unwrap_or(matches!(mode, VirtualSwitchMode::Nat | VirtualSwitchMode::HostOnly));

    let dhcp_start: Ipv4Addr = match dhcp_range_start {
        Some(s) if !s.trim().is_empty() => s.parse().map_err(|e| ApiError::new("INVALID_DHCP_START", format!("Invalid DHCP start IP: {e}")))?,
        _ => match mode {
            VirtualSwitchMode::Nat => "192.168.128.128".parse().unwrap(),
            VirtualSwitchMode::HostOnly => "192.168.192.128".parse().unwrap(),
            VirtualSwitchMode::Bridged => "192.168.1.100".parse().unwrap(),
            VirtualSwitchMode::Internal => "10.0.0.10".parse().unwrap(),
        },
    };

    let dhcp_end: Ipv4Addr = match dhcp_range_end {
        Some(e) if !e.trim().is_empty() => e.parse().map_err(|err| ApiError::new("INVALID_DHCP_END", format!("Invalid DHCP end IP: {err}")))?,
        _ => match mode {
            VirtualSwitchMode::Nat => "192.168.128.254".parse().unwrap(),
            VirtualSwitchMode::HostOnly => "192.168.192.254".parse().unwrap(),
            VirtualSwitchMode::Bridged => "192.168.1.200".parse().unwrap(),
            VirtualSwitchMode::Internal => "10.0.0.254".parse().unwrap(),
        },
    };

    let id = state
        .network
        .create_switch_detailed(
            name.clone(),
            mode,
            parsed_subnet,
            parsed_gw,
            is_dhcp,
            dhcp_start,
            dhcp_end,
            adapter_name,
        )
        .map_err(|e| ApiError::new("NETWORK_ERROR", e.to_string()))?;
    state.push_log("INFO", "network", format!("Virtual switch '{name}' ({mode:?}) created successfully"));
    Ok(id.to_string())
}

/// Update an existing virtual switch.
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn update_switch(
    name: String,
    mode: VirtualSwitchMode,
    subnet: String,
    gateway: String,
    dhcp_enabled: bool,
    dhcp_range_start: String,
    dhcp_range_end: String,
    adapter_name: Option<String>,
    state: State<'_, AppState>,
) -> ApiResult<()> {
    let parsed_subnet: Ipv4Net = subnet.parse().map_err(|e| ApiError::new("INVALID_SUBNET", format!("Invalid subnet CIDR: {e}")))?;
    let parsed_gw: Ipv4Addr = gateway.parse().map_err(|e| ApiError::new("INVALID_GATEWAY", format!("Invalid gateway IP: {e}")))?;
    let dhcp_start: Ipv4Addr = dhcp_range_start.parse().map_err(|e| ApiError::new("INVALID_DHCP_START", format!("Invalid DHCP start IP: {e}")))?;
    let dhcp_end: Ipv4Addr = dhcp_range_end.parse().map_err(|e| ApiError::new("INVALID_DHCP_END", format!("Invalid DHCP end IP: {e}")))?;

    state
        .network
        .update_switch(
            &name,
            mode,
            parsed_subnet,
            parsed_gw,
            dhcp_enabled,
            dhcp_start,
            dhcp_end,
            adapter_name,
        )
        .map_err(|e| ApiError::new("NETWORK_ERROR", e.to_string()))?;
    state.push_log("INFO", "network", format!("Virtual switch '{name}' updated"));
    Ok(())
}

/// Delete a virtual switch.
#[tauri::command]
pub async fn delete_switch(name: String, state: State<'_, AppState>) -> ApiResult<()> {
    state.network.delete_switch(&name).map_err(|e| ApiError::new("NETWORK_ERROR", e.to_string()))?;
    state.push_log("WARN", "network", format!("Virtual switch '{name}' deleted"));
    Ok(())
}

/// Enumerate physical network adapters available on host.
#[tauri::command]
pub async fn list_physical_adapters(state: State<'_, AppState>) -> ApiResult<Vec<String>> {
    Ok(state.network.list_physical_adapters())
}
