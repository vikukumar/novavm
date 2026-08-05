# Changelog

All notable changes to NovaVM will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-08-04

### Added
- **NovaVM Desktop Application (Tauri v2)**: Cross-platform enterprise virtualization manager for Windows, Linux, and macOS.
- **Multi-Hypervisor Backend Architecture**: Native support for Windows Hypervisor Platform (WHP), Linux KVM (`/dev/kvm`), and Apple Virtualization Framework (`Virtualization.framework`).
- **NovaDisk Storage Engine**: High-performance thin-provisioned disk images with zstd bulk compression, AES-256-GCM encryption, and CoW overlay graph snapshots.
- **Dynamic Memory Management**: Real-time host and guest memory tracking, ballooning support, and memory pressure detection algorithms.
- **Virtual Networking Manager**: NAT, Host-Only, and Bridged virtual switches with integrated DHCP leases.
- **Real-time Monitoring**: System host metrics & VM resource tracking ring-buffer engine with live UI visualizations.
- **Frontend Dashboard**: Sleek Dark/Light glassmorphism user interface built with React 18, TypeScript, Tailwind CSS, Lucide icons, Framer Motion, and Recharts.
- **Automated Release Pipeline**: GitHub Actions CI/CD with auto-incrementing patch versions, SHA256 checksum generation, and multi-platform installers (MSI, EXE, DEB, RPM, AppImage, DMG).
