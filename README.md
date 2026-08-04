# NovaVM

**Lightweight, enterprise-grade cross-platform virtualization manager**

[![CI](https://github.com/vikukumar/novavm/actions/workflows/ci.yml/badge.svg)](https://github.com/vikukumar/novavm/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

---

## Overview

NovaVM is a production-ready desktop virtualization manager that uses each operating system's native hypervisor backend:

| Platform | Backend |
|----------|---------|
| Windows  | Windows Hypervisor Platform (WHP / Hyper-V) |
| Linux    | KVM (`/dev/kvm`) |
| macOS    | Apple Virtualization Framework |

The desktop application is built with **Tauri 2** + **React 18** + **TypeScript** + **Tailwind CSS** + **shadcn/ui**. All core logic is in native Rust.

## Architecture

```
novavm/
├── engine/          VM lifecycle state machine
├── hypervisor/      Platform-native hypervisor abstraction (WHP/KVM/AVF)
├── scheduler/       CPU scheduling & overcommit
├── memory/          Dynamic memory, ballooning, dedup, hugepages
├── storage/         NovaDisk format, CoW snapshots, AES-256-GCM, zstd
├── network/         Virtual switches, DHCP, DNS, NAT/bridged/host-only
├── snapshot/        Snapshot orchestration
├── monitor/         Real-time host & VM metrics
├── agent/           In-guest agent protocol
├── api/             Unified API facade for Tauri commands
├── sdk/             Public plugin SDK
├── src-tauri/       Tauri desktop application shell
├── frontend/        React + TypeScript + Tailwind UI
├── docs/            mdBook documentation
└── installer/       Platform-specific installer configs
```

## Prerequisites

- Rust stable ≥ 1.78
- Node.js ≥ 18
- `cargo-tauri` CLI: `cargo install tauri-cli --version "^2" --locked`
- **Windows**: Enable Hyper-V / Windows Hypervisor Platform in Windows Features
- **Linux**: KVM must be available (`ls /dev/kvm`)
- **macOS**: macOS 11.0+ (Apple Silicon or Intel)

## Development

```bash
# Clone
git clone https://github.com/vikukumar/novavm
cd novavm

# Install frontend dependencies
cd frontend && npm install && cd ..

# Run in development mode (hot-reload)
cargo tauri dev

# Run all Rust tests
cargo test --workspace

# Check formatting & lints
cargo fmt --all --check
cargo clippy --workspace -- -D warnings
```

## Building

```bash
# Build release
cargo tauri build

# Rust workspace only
cargo build --workspace --release

# Frontend only
cd frontend && npm run build
```

## Features

### Core
- VM creation wizard with multi-step configuration
- Full VM lifecycle: create, start, pause, resume, stop, reset, destroy
- VM cloning and import/export
- NAT, bridged, host-only, and internal networking
- Virtual TPM 2.0 and Secure Boot (platform-dependent)
- Shared folders and clipboard sharing
- USB device redirection (platform-dependent)

### Storage (NovaDisk)
- Custom virtual disk container format
- Thin provisioning
- Copy-on-write snapshots with incremental backup
- AES-256-GCM encryption
- zstd compression

### Monitoring
- Real-time CPU, RAM, disk I/O, network I/O per VM and host
- Ring-buffer history for charts (300 samples)
- Configurable polling interval

### Frontend
- Enterprise dashboard with live metrics charts
- VM list with search, filtering, group/tag support
- Multi-step VM creation wizard
- Dark, light, and system themes
- ⌘K command palette
- Responsive layouts, Framer Motion animations
- Keyboard shortcuts

### Security
- Code-signed builds (via Tauri)
- Encrypted update delivery
- Audit logging
- RBAC-ready API design

## Plugin Development

Implement the `NovaPlugin` trait from the `sdk` crate:

```rust
use nova_sdk::{NovaPlugin, PluginContext, PluginError, PluginMetadata};

pub struct MyPlugin;

impl NovaPlugin for MyPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "com.example.my-plugin".to_owned(),
            name: "My Plugin".to_owned(),
            version: "1.0.0".to_owned(),
            description: "Example NovaVM plugin".to_owned(),
            author: "You".to_owned(),
            min_novavm_version: "0.1.0".to_owned(),
        }
    }

    fn on_load(&mut self, ctx: &PluginContext) -> Result<(), PluginError> {
        println!("Plugin loaded at {}", ctx.data_dir.display());
        Ok(())
    }

    fn on_unload(&mut self) {}
}
```

## License

Apache License 2.0 — see [LICENSE](LICENSE).
