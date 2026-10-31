#!/usr/bin/env bash
# Install Pinnacle from source on Fedora Workstation 44, next to GNOME.
#
# What it does (each step is safe to re-run):
#   1. installs build and runtime packages with dnf
#   2. clones Pinnacle into ~/src/pinnacle at $PINNACLE_REF and builds it
#   3. installs the compositor and its session files system-wide (sudo)
#   4. installs the Lua API so `pinnacle client` works (optional, see --no-lua)
#   5. links this repo to ~/.config/pinnacle, pre-builds the config and installs `pctl`
#
# Afterwards, log out and pick "Pinnacle" from the gear menu on the GDM login screen.
#
# Environment overrides:
#   PINNACLE_REPO  git URL to build from (set this to your fork once you hack on the compositor)
#   PINNACLE_REF   tag/branch/commit; must match the pinnacle-api tag in Cargo.toml
#   PINNACLE_SRC   checkout location (default ~/src/pinnacle)

set -euo pipefail

PINNACLE_REPO="${PINNACLE_REPO:-https://github.com/pinnacle-comp/pinnacle.git}"
PINNACLE_REF="${PINNACLE_REF:-v0.2.4}"
PINNACLE_SRC="${PINNACLE_SRC:-$HOME/src/pinnacle}"
CONFIG_REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WITH_LUA=1

for arg in "$@"; do
    case "$arg" in
        --no-lua) WITH_LUA=0 ;;
        -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
        *) echo "unknown option: $arg" >&2; exit 2 ;;
    esac
done

step() { printf '\n\033[1;34m==> %s\033[0m\n' "$*"; }

# ---------------------------------------------------------------------------
step "Installing packages"

BUILD_DEPS=(
    rust cargo clippy rustfmt gcc pkgconf-pkg-config just git protobuf-compiler
    wayland-devel libxkbcommon-devel systemd-devel libinput-devel mesa-libgbm-devel
    libseat-devel mesa-libEGL-devel libdisplay-info-devel
)
RUNTIME_DEPS=(
    xorg-x11-server-Xwayland vulkan-loader mesa-vulkan-drivers
    xdg-desktop-portal-wlr xdg-desktop-portal-gtk gnome-keyring polkit-gnome
    alacritty fuzzel waybar mako brightnessctl playerctl wireplumber
)
LUA_DEPS=(lua lua-devel luarocks openssl-devel m4)

pkgs=("${BUILD_DEPS[@]}" "${RUNTIME_DEPS[@]}")
[[ $WITH_LUA == 1 ]] && pkgs+=("${LUA_DEPS[@]}")
sudo dnf install -y "${pkgs[@]}"

rust_minor="$(rustc --version | sed -E 's/rustc 1\.([0-9]+).*/\1/')"
if (( rust_minor < 88 )); then
    echo "Pinnacle needs Rust >= 1.88, found $(rustc --version). Install rustup and retry." >&2
    exit 1
fi

# ---------------------------------------------------------------------------
step "Fetching Pinnacle $PINNACLE_REF"

if [[ -d "$PINNACLE_SRC/.git" ]]; then
    git -C "$PINNACLE_SRC" fetch --tags origin
else
    mkdir -p "$(dirname "$PINNACLE_SRC")"
    git clone "$PINNACLE_REPO" "$PINNACLE_SRC"
fi
git -C "$PINNACLE_SRC" checkout --quiet "$PINNACLE_REF"

# ---------------------------------------------------------------------------
step "Building Pinnacle (release; first build takes a while)"

(cd "$PINNACLE_SRC" && just build --release --locked)

# ---------------------------------------------------------------------------
step "Installing compositor and session files (sudo)"

res="$PINNACLE_SRC/resources"
sudo install -Dm755 "$PINNACLE_SRC/target/release/pinnacle" /usr/local/bin/pinnacle
sudo install -Dm755 "$res/pinnacle-session"                  /usr/local/bin/pinnacle-session
sudo install -Dm644 "$res/pinnacle-portals.conf"             /usr/share/xdg-desktop-portal/pinnacle-portals.conf
sudo install -Dm644 "$res/pinnacle-shutdown.target"          /usr/lib/systemd/user/pinnacle-shutdown.target

# The upstream unit and session file assume /usr/bin; point them at /usr/local/bin instead,
# so nothing here collides with a future Fedora package.
sed 's|/usr/bin/pinnacle|/usr/local/bin/pinnacle|' "$res/pinnacle.service" \
    | sudo tee /usr/lib/systemd/user/pinnacle.service >/dev/null
sed 's|^Exec=.*|Exec=/usr/local/bin/pinnacle-session|' "$res/pinnacle.desktop" \
    | sudo tee /usr/share/wayland-sessions/pinnacle.desktop >/dev/null
systemctl --user daemon-reload

# ---------------------------------------------------------------------------
if [[ $WITH_LUA == 1 ]]; then
    step "Installing the Lua API for \`pinnacle client\`"
    if ! (cd "$PINNACLE_SRC" && just install); then
        echo "Lua API install failed; pctl still works. Re-run later or use --no-lua." >&2
    fi
fi

# ---------------------------------------------------------------------------
step "Linking this repo as ~/.config/pinnacle"

target="$HOME/.config/pinnacle"
if [[ -L "$target" ]]; then
    ln -sfn "$CONFIG_REPO" "$target"
elif [[ -e "$target" ]]; then
    backup="$target.bak.$(date +%Y%m%d%H%M%S)"
    mv "$target" "$backup"
    echo "Existing config moved to $backup"
    ln -s "$CONFIG_REPO" "$target"
else
    mkdir -p "$HOME/.config"
    ln -s "$CONFIG_REPO" "$target"
fi

# ---------------------------------------------------------------------------
step "Pre-building the config and installing pctl"

# Building now means the first login does not wait for a full compile.
(cd "$CONFIG_REPO" && cargo build --package pinnacle-config)
cargo install --path "$CONFIG_REPO/pctl"

case ":$PATH:" in
    *":$HOME/.cargo/bin:"*) ;;
    *) echo "Add ~/.cargo/bin to your PATH so \`pctl\` is found." ;;
esac

step "Done"
echo "Try it nested first:  $CONFIG_REPO/scripts/nested.sh"
echo "Then log out and choose \"Pinnacle\" on the login screen."
