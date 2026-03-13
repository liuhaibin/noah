#!/usr/bin/env bash
# Noah Setup Script — Installs all necessary system dependencies for Tauri on Linux.
#
# Usage:
#   ./scripts/setup-linux.sh
#   NOAH_DISTROBOX_NAME=rust-dev ./scripts/setup-linux.sh --distrobox

set -euo pipefail

# Dependencies required for Tauri 2.0
# We map these to the appropriate package names for each package manager.
DEPS_APT=(
    "build-essential" "curl" "wget" "file" "pkg-config" "libssl-dev" "libgtk-3-dev"
    "libayatana-appindicator3-dev" "librsvg2-dev" "libcairo2-dev" "libwebkit2gtk-4.1-dev"
    "libglib2.0-dev" "libpango1.0-dev" "libatk1.0-dev" "libgdk-pixbuf2.0-dev"
    "libsoup-3.0-dev" "javascriptcoregtk-4.1-dev" "patchelf"
)

DEPS_DNF=(
    "gcc" "gcc-c++" "make" "curl" "wget" "file" "pkg-config" "openssl-devel"
    "gtk3-devel" "libayatana-appindicator-gtk3-devel" "librsvg2-devel" "cairo-devel"
    "webkit2gtk4.1-devel" "glib2-devel" "pango-devel" "atk-devel"
    "gdk-pixbuf2-devel" "libsoup3-devel" "javascriptcoregtk4.1-devel" "patchelf"
)

install_native() {
    if command -v apt-get &>/dev/null; then
        echo "Installing dependencies on host (apt)..."
        sudo apt-get update
        sudo apt-get install -y "${DEPS_APT[@]}"
    elif command -v dnf &>/dev/null; then
        echo "Installing dependencies on host (dnf)..."
        sudo dnf install -y "${DEPS_DNF[@]}"
    else
        echo "ERROR: Unsupported package manager. Please install dependencies manually."
        exit 1
    fi
}

install_distrobox() {
    local container="${NOAH_DISTROBOX_NAME:-noah-build}"
    echo "Installing dependencies in distrobox [$container]..."
    # Robustly check if the container name exists in the NAME column (second field)
    if ! distrobox list 2>/dev/null | awk -F'|' 'NR>1 {print $2}' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | grep -qx "$container"; then
        echo "ERROR: Distrobox '$container' not found."
        exit 1
    fi

    # Determine which package manager to use inside the container
    if distrobox enter "$container" -- command -v apt-get &>/dev/null; then
        echo "Using apt inside $container..."
        distrobox enter "$container" -- sudo apt-get update
        distrobox enter "$container" -- sudo apt-get install -y "${DEPS_APT[@]}"
    elif distrobox enter "$container" -- command -v dnf &>/dev/null; then
        echo "Using dnf inside $container..."
        # Use -y and optionally --skip-unavailable to be non-interactive and robust
        distrobox enter "$container" -- sudo dnf install -y "${DEPS_DNF[@]}"
    else
        echo "ERROR: Could not determine package manager inside $container."
        exit 1
    fi
}

# Main
if [[ "${1:-}" == "--distrobox" ]]; then
    install_distrobox
else
    install_native
fi

echo "✅ Dependencies installed successfully."
