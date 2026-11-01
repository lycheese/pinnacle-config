#!/usr/bin/env bash
# Install Pinnacle from source on Pop!_OS 24.04 (or Ubuntu 24.04), next to COSMIC/GNOME.
#
# Steps (each safe to re-run):
#   1. installs build and runtime packages with apt
#   2. installs Rust with rustup if the toolchain is missing or older than 1.88
#      (the distro's rustc is too old for Pinnacle)
#   3. installs Helix into ~/.local from its release tarball (scripts/install-helix.sh)
#   4. clones Pinnacle into ~/src/pinnacle at $PINNACLE_REF and builds it
#   5. installs the compositor and its session files under /usr/local (sudo)
#   6. installs the Lua API so `pinnacle client` works (skip with --no-lua)
#   7. links this repo to ~/.config/pinnacle, pre-builds the config, installs `pctl`, creates
#      the notes and agent folders and installs Claude Code (user-local)
#
# Options:
#   --no-lua          skip the Lua API
#   --packages-only   only install packages, then stop
#
# Overrides (PINNACLE_REPO, PINNACLE_REF, PINNACLE_SRC) are described in scripts/lib/common.sh.

set -euo pipefail
# shellcheck source=scripts/lib/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"
parse_args "$(sed -n '2,19p' "$0" | sed 's/^# \{0,1\}//')" "$@"

# Pop!_OS 22.04 and other older bases lack libdisplay-info and recent Wayland libraries.
# shellcheck disable=SC1091
. /etc/os-release
version="${VERSION_ID:-0}"
if (( ${version%%.*} < 24 )); then
    echo "This script needs Pop!_OS/Ubuntu 24.04 or newer (found ${PRETTY_NAME:-unknown})." >&2
    exit 1
fi

BUILD_DEPS=(
    build-essential pkg-config git curl protobuf-compiler
    libwayland-dev libxkbcommon-dev libudev-dev libsystemd-dev libinput-dev libgbm-dev
    libseat-dev libegl-dev libdisplay-info-dev
)
RUNTIME_DEPS=(
    xwayland libvulkan1 mesa-vulkan-drivers
    xdg-desktop-portal-wlr xdg-desktop-portal-gtk gnome-keyring policykit-1-gnome
    alacritty fuzzel waybar mako-notifier brightnessctl playerctl wireplumber
    fonts-font-awesome grim slurp wl-clipboard libnotify-bin fd-find
)
LUA_DEPS=(lua5.4 liblua5.4-dev luarocks libssl-dev m4)

step "Installing packages"
pkgs=("${BUILD_DEPS[@]}" "${RUNTIME_DEPS[@]}")
[[ $WITH_LUA == 1 ]] && pkgs+=("${LUA_DEPS[@]}")
sudo apt-get update
sudo apt-get install -y "${pkgs[@]}"

# luarocks pulls in lua5.1, which then wins the `lua` alternative. `pinnacle client` runs plain
# `lua` and the API is built for 5.4.
if [[ $WITH_LUA == 1 ]]; then
    sudo update-alternatives --set lua-interpreter /usr/bin/lua5.4
fi

[[ $PACKAGES_ONLY == 1 ]] && exit 0

# ---------------------------------------------------------------------------
step "Checking the Rust toolchain"

# shellcheck disable=SC1091
[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

if ! command -v rustup >/dev/null 2>&1 && ! { command -v rustc >/dev/null 2>&1 && require_rust 2>/dev/null; }; then
    echo "Installing Rust via rustup (user-local, in ~/.cargo and ~/.rustup)."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal \
        --component clippy,rustfmt
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
elif command -v rustup >/dev/null 2>&1; then
    rustup update stable
fi

# Pinnacle's justfile uses features from recent `just` releases, newer than Ubuntu 24.04 ships.
just_minor="$(just --version 2>/dev/null | sed -nE 's/^just 1\.([0-9]+).*/\1/p')"
if [[ -z "$just_minor" ]] || (( just_minor < 36 )); then
    cargo install --locked just
fi

step "Installing Helix (user-local, from the release tarball)"
"$(dirname "${BASH_SOURCE[0]}")/install-helix.sh"

run_common_steps
