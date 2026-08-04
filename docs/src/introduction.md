# Introduction

**NovaVM** is a lightweight, enterprise-grade virtualization manager for Windows, Linux, and macOS.

## Key Features

- **Native hypervisor integration**: Uses Windows Hypervisor Platform (WHP/Hyper-V), KVM (Linux), and Apple Virtualization Framework (macOS).
- **Modern desktop UI**: Built with Tauri 2, React 18, TypeScript, Tailwind CSS, and shadcn/ui.
- **Production-ready storage**: NovaDisk format with thin provisioning, CoW snapshots, AES-256-GCM encryption, and zstd compression.
- **Full networking stack**: Virtual switches with NAT, bridged, host-only, and internal modes; embedded DHCP and DNS.
- **Real-time monitoring**: Live host and per-VM CPU, memory, disk, and network metrics with ring-buffer history.
- **Plugin system**: Stable Rust SDK for third-party integrations.
- **Enterprise security**: Audit logs, encrypted secrets, RBAC-ready API, signed updates.

## Architecture

```
┌─────────────────────────────────────────────┐
│              NovaVM Desktop App             │
│      Tauri 2 · React 18 · TypeScript        │
├─────────────────────────────────────────────┤
│              Tauri IPC Commands             │
│                  (api crate)                │
├────────┬─────────┬──────────┬───────────────┤
│ engine │scheduler│  memory  │   monitor     │
├────────┴─────────┴──────────┴───────────────┤
│              hypervisor (trait)             │
│   WHP (Windows) │ KVM (Linux) │ AVF (macOS) │
├─────────────────┬───────────────────────────┤
│    storage      │          network          │
│  NovaDisk·CoW   │  Virtual Switch·DHCP·DNS  │
└─────────────────┴───────────────────────────┘
```

## Quick Links

- [Installation](./user/installation.md)
- [Quick Start](./user/quickstart.md)
- [Building from Source](./dev/building.md)
