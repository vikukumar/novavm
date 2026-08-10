const fs = require('fs');
const path = require('path');

const version = process.argv[2] || '1.0.0';

const notes = `# NovaVM Release Notes 🚀

We are excited to announce NovaVM! This release brings expanded cross-platform architecture support for **Linux ARM64** and **Windows ARM64**, key hypervisor kernel stability improvements across KVM and WHP backends, zero compiler warnings, and improved automated multi-platform release bundle packaging.

---

## 🌟 Key Highlights & Feature Changes

### 🖥️ Expanded Architecture Support
- **Linux ARM64 (\`aarch64-unknown-linux-gnu\`):** Added full native Debian (\`.deb\`) and AppImage bundle targets with GCC cross-compilation pipeline support.
- **Windows ARM64 (\`aarch64-pc-windows-msvc\`):** Added native ARM64 Windows NSIS installer setup generation.
- **Universal Multi-Platform Coverage:** NovaVM now builds native packages across 6 major target architectures (Linux x86_64/ARM64, Windows x86_64/ARM64, macOS Intel/Apple Silicon).

### ⚡ Hypervisor & Engine Improvements
- **Linux KVM Backend:** Fixed mutable \`VcpuFd\` borrow contract inside \`kvm_vcpu_thread\` (\`vcpu.run()\`), ensuring robust hardware-assisted execution on Linux hosts.
- **Null Backend & Test Suite:** Synchronized \`CreateVmRequest\` parameter fields across test fixtures (\`id: Option<Uuid>\`).
- **Code Hygiene:** Cleaned encoding mojibake/corrupted UTF-8 box characters across \`kvm.rs\`, \`whp.rs\`, \`qemu.rs\`, and \`null.rs\`.
- **Warning-Free Compilation:** Resolved unused import (\`FramebufferFrame\`) and platform-conditional parameter warnings in \`send_vm_input\`.

### 📦 Release Pipeline & Packaging
- **Automatic Workspace Bundle Discovery:** Fixed bundle resolution for root Cargo workspace builds (\`target/release/bundle/\`).
- **Clean Release Generation:** Optimized CI workflow to generate release notes once per tag without duplicated changelog blocks across parallel matrix jobs.

---

## 💾 Download & Installation Packages

| Platform | Architecture | Package Format | Binary File |
| :--- | :--- | :--- | :--- |
| **Windows** | \`x86_64\` (64-bit) | MSI Installer / NSIS Setup | \`NovaVM_*_x64-setup.exe\` |
| **Windows** | \`aarch64\` (ARM64) | NSIS Setup | \`NovaVM_*_arm64-setup.exe\` |
| **Linux** | \`x86_64\` (64-bit) | Debian Package / AppImage | \`NovaVM_*_amd64.deb\`, \`NovaVM_*_amd64.AppImage\` |
| **Linux** | \`aarch64\` (ARM64) | Debian Package / AppImage | \`NovaVM_*_arm64.deb\`, \`NovaVM_*_arm64.AppImage\` |
| **macOS** | \`x86_64\` (Intel) | Disk Image / App Bundle | \`NovaVM_*_x64.dmg\` |
| **macOS** | \`aarch64\` (Apple Silicon) | Disk Image / App Bundle | \`NovaVM_*_aarch64.dmg\` |
`;

fs.writeFileSync(path.join(process.cwd(), 'RELEASE_NOTES.md'), notes, 'utf8');
console.log(`Generated RELEASE_NOTES.md for version ${version}`);
