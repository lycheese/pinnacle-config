#!/usr/bin/env bash
# Install Pinnacle from source on Fedora Workstation 44, next to GNOME.
#
# Steps (each safe to re-run):
#   1. installs build and runtime packages with dnf
#   2. clones Pinnacle into ~/src/pinnacle at $PINNACLE_REF and builds it
#   3. installs the compositor and its session files under /usr/local (sudo)
#   4. installs the Lua API so `pinnacle client` works (skip with --no-lua)
#   5. links this repo to ~/.config/pinnacle, pre-builds the config and installs `pctl`
#
# Options:
#   --no-lua          skip the Lua API
#   --packages-only   only install packages, then stop
#
# Overrides (PINNACLE_REPO, PINNACLE_REF, PINNACLE_SRC) are described in scripts/lib/common.sh.

set -euo pipefail
# shellcheck source=scripts/lib/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/lib/common.sh"
parse_args "$(sed -n '2,16p' "$0" | sed 's/^# \{0,1\}//')" "$@"

BUILD_DEPS=(
    rust cargo clippy rustfmt gcc pkgconf-pkg-config just git protobuf-compiler
    wayland-devel libxkbcommon-devel systemd-devel libinput-devel mesa-libgbm-devel
    libseat-devel mesa-libEGL-devel libdisplay-info-devel
)
RUNTIME_DEPS=(
    xorg-x11-server-Xwayland vulkan-loader mesa-vulkan-drivers
    xdg-desktop-portal-wlr xdg-desktop-portal-gtk gnome-keyring mate-polkit
    alacritty fuzzel waybar mako brightnessctl playerctl wireplumber
    fontawesome4-fonts grim slurp wl-clipboard libnotify
)
LUA_DEPS=(lua lua-devel luarocks openssl-devel m4)

step "Installing packages"
pkgs=("${BUILD_DEPS[@]}" "${RUNTIME_DEPS[@]}")
[[ $WITH_LUA == 1 ]] && pkgs+=("${LUA_DEPS[@]}")
sudo dnf install -y "${pkgs[@]}"

[[ $PACKAGES_ONLY == 1 ]] && exit 0

if ! require_rust; then
    echo "Install a newer toolchain with rustup (https://rustup.rs) and retry." >&2
    exit 1
fi

run_common_steps
