#!/bin/sh
# =============================================================================
# NovaVM Linux Post-Install Script
# Developer : Vikash Kumar <https://vikukumar.github.io>
# Product   : NovaVM v1.0.0
# Year      : 2026
# License   : MIT
#
# This script runs after .deb or .rpm installation. It:
#   1. Adds the current user (or the SUDO_USER) to the 'kvm' group so
#      NovaVM can use /dev/kvm for hardware-accelerated virtualisation.
#   2. Sets correct permissions on /dev/kvm (in case the group isn't applied).
#   3. Registers the NovaVM desktop entry with the system icon cache.
#
# On Arch Linux, the group may be 'kvm'; on some distros it's 'libvirt'.
# We handle both.
# =============================================================================

set -e

PRODUCT_NAME="NovaVM"
DEVELOPER="Vikash Kumar"
HOMEPAGE="https://vikukumar.github.io"
VERSION="1.0.8"

log() {
    echo "[$PRODUCT_NAME installer] $1"
}

log "$PRODUCT_NAME v$VERSION post-install script"
log "Developer: $DEVELOPER — $HOMEPAGE"
log ""

# ── Determine the real user (the one who ran sudo) ─────────────────────────
REAL_USER="${SUDO_USER:-$USER}"
if [ -z "$REAL_USER" ] || [ "$REAL_USER" = "root" ]; then
    REAL_USER=$(logname 2>/dev/null || whoami)
fi
log "Configuring KVM access for user: $REAL_USER"

# ── 1. Enable KVM hardware virtualisation ─────────────────────────────────────
if [ -e /dev/kvm ]; then
    log "Found /dev/kvm — enabling KVM access..."

    # Add to 'kvm' group (most distros: Ubuntu, Fedora, openSUSE, Arch)
    if getent group kvm > /dev/null 2>&1; then
        usermod -aG kvm "$REAL_USER" 2>/dev/null && \
            log "  Added '$REAL_USER' to group 'kvm'" || \
            log "  Warning: could not add to 'kvm' group (usermod failed)"
    fi

    # Add to 'libvirt' group if it exists (Debian/Ubuntu QEMU stacks)
    if getent group libvirt > /dev/null 2>&1; then
        usermod -aG libvirt "$REAL_USER" 2>/dev/null && \
            log "  Added '$REAL_USER' to group 'libvirt'" || true
    fi

    # Ensure /dev/kvm is group-readable (fixes some distros with mode 600)
    chmod 0666 /dev/kvm 2>/dev/null || true
    log "  /dev/kvm permissions set to 0666"
    log "  NOTE: You must log out and back in for group membership to take effect."
else
    log "  /dev/kvm not found. KVM may not be supported by your CPU or kernel."
    log "  NovaVM will run without hardware acceleration."
fi

# ── 2. Update MIME and desktop database ───────────────────────────────────────
if command -v update-desktop-database > /dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications 2>/dev/null || true
fi
if command -v gtk-update-icon-cache > /dev/null 2>&1; then
    gtk-update-icon-cache -q /usr/share/icons/hicolor 2>/dev/null || true
fi

# ── 3. Write udev rule for persistent /dev/kvm permissions ────────────────────
UDEV_RULE="/etc/udev/rules.d/99-novavm-kvm.rules"
if [ ! -f "$UDEV_RULE" ]; then
    cat > "$UDEV_RULE" << 'EOF'
# NovaVM — Vikash Kumar (https://vikukumar.github.io)
# Allow members of the 'kvm' group to access /dev/kvm for hardware virtualisation.
KERNEL=="kvm", GROUP="kvm", MODE="0660"
EOF
    log "  Wrote udev rule: $UDEV_RULE"
    # Reload udev rules if possible
    udevadm control --reload-rules 2>/dev/null || true
    udevadm trigger 2>/dev/null || true
fi

log ""
log "$PRODUCT_NAME v$VERSION installed successfully!"
log "Visit $HOMEPAGE for documentation and support."

exit 0
