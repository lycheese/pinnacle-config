# shellcheck shell=bash
# Variables like PACKAGES_ONLY are set here but read by the distro scripts.
# shellcheck disable=SC2034
# Shared install steps, sourced by scripts/install-<distro>.sh.
# The distro scripts install packages and a Rust toolchain; everything after that lives here,
# so both installers build and install Pinnacle the same way.
#
# Environment overrides:
#   PINNACLE_REPO  git URL to build from (set this to your fork once you hack on the compositor)
#   PINNACLE_REF   tag/branch/commit; must speak the same protocol as the pinnacle-api pin in
#                  Cargo.toml (v0.2.4, or the fork's fix/signal-stream-end branch it points at)
#   PINNACLE_SRC   checkout location (default ~/src/pinnacle)

PINNACLE_REPO="${PINNACLE_REPO:-https://github.com/pinnacle-comp/pinnacle.git}"
PINNACLE_REF="${PINNACLE_REF:-v0.2.4}"
PINNACLE_SRC="${PINNACLE_SRC:-$HOME/src/pinnacle}"
CONFIG_REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

WITH_LUA=1
PACKAGES_ONLY=0
WITH_LOCAL_AI=0

step() { printf '\n\033[1;34m==> %s\033[0m\n' "$*"; }

# Parses the options every installer accepts. $1 is the help text.
parse_args() {
    local help="$1"
    shift
    for arg in "$@"; do
        case "$arg" in
            --no-lua) WITH_LUA=0 ;;
            --packages-only) PACKAGES_ONLY=1 ;;
            --with-local-ai) WITH_LOCAL_AI=1 ;;
            -h|--help) printf '%s\n' "$help"; exit 0 ;;
            *) echo "unknown option: $arg" >&2; exit 2 ;;
        esac
    done
}

require_rust() {
    local minor
    minor="$(rustc --version | sed -E 's/rustc 1\.([0-9]+).*/\1/')"
    if (( minor < 88 )); then
        echo "Pinnacle needs Rust >= 1.88, found $(rustc --version)." >&2
        return 1
    fi
}

fetch_and_build_pinnacle() {
    step "Fetching Pinnacle $PINNACLE_REF"
    if [[ -d "$PINNACLE_SRC/.git" ]]; then
        git -C "$PINNACLE_SRC" fetch --tags origin
    else
        mkdir -p "$(dirname "$PINNACLE_SRC")"
        git clone "$PINNACLE_REPO" "$PINNACLE_SRC"
    fi
    git -C "$PINNACLE_SRC" checkout --quiet "$PINNACLE_REF"

    step "Building Pinnacle (release; first build takes a while)"
    (cd "$PINNACLE_SRC" && just build --release --locked)
}

# Installs under /usr/local so nothing collides with a future distro package.
install_session_files() {
    step "Installing compositor and session files (sudo)"
    local res="$PINNACLE_SRC/resources"
    sudo install -Dm755 "$PINNACLE_SRC/target/release/pinnacle" /usr/local/bin/pinnacle
    sudo install -Dm755 "$res/pinnacle-session"                  /usr/local/bin/pinnacle-session
    sudo install -Dm644 "$res/pinnacle-portals.conf"             /usr/share/xdg-desktop-portal/pinnacle-portals.conf
    sudo install -Dm644 "$res/pinnacle-shutdown.target"          /usr/lib/systemd/user/pinnacle-shutdown.target

    # The upstream unit and session file assume /usr/bin.
    sed 's|/usr/bin/pinnacle|/usr/local/bin/pinnacle|' "$res/pinnacle.service" \
        | sudo tee /usr/lib/systemd/user/pinnacle.service >/dev/null
    sed 's|^Exec=.*|Exec=/usr/local/bin/pinnacle-session|' "$res/pinnacle.desktop" \
        | sudo tee /usr/share/wayland-sessions/pinnacle.desktop >/dev/null
    systemctl --user daemon-reload
}

install_lua_api() {
    [[ $WITH_LUA == 1 ]] || return 0
    step "Installing the Lua API for \`pinnacle client\`"
    # The justfile sets LUA_PATH to luarocks' user paths plus our LUA_PATH. Without `;;` (Lua's
    # default path) luarocks cannot load its own modules from /usr/share/lua.
    if ! (cd "$PINNACLE_SRC" && LUA_PATH="${LUA_PATH:-};;" LUA_CPATH="${LUA_CPATH:-};;" just install); then
        echo "Lua API install failed; pctl still works. Re-run later or use --no-lua." >&2
        return 0
    fi

    # `pinnacle client` runs `lua` with the inherited LUA_PATH, so the session needs the
    # luarocks user tree. environment.d reaches the systemd user manager, which starts Pinnacle.
    local envfile="$HOME/.config/environment.d/50-pinnacle-lua.conf"
    mkdir -p "$(dirname "$envfile")"
    cat > "$envfile" <<'EOF'
# Written by pinnacle-config's installer: lets `pinnacle client` find the Lua API.
LUA_PATH=${HOME}/.luarocks/share/lua/5.4/?.lua;${HOME}/.luarocks/share/lua/5.4/?/init.lua;;
LUA_CPATH=${HOME}/.luarocks/lib/lua/5.4/?.so;;
EOF
    echo "Wrote $envfile (takes effect at next login)."
}

link_config() {
    step "Linking this repo as ~/.config/pinnacle"
    local target="$HOME/.config/pinnacle"
    if [[ -L "$target" ]]; then
        ln -sfn "$CONFIG_REPO" "$target"
    elif [[ -e "$target" ]]; then
        local backup
        backup="$target.bak.$(date +%Y%m%d%H%M%S)"
        mv "$target" "$backup"
        echo "Existing config moved to $backup"
        ln -s "$CONFIG_REPO" "$target"
    else
        mkdir -p "$HOME/.config"
        ln -s "$CONFIG_REPO" "$target"
    fi
}

# Ubuntu enables waybar.service for every graphical session. The config already starts waybar,
# so the unit would add a second bar in Pinnacle (and a stray one in COSMIC/GNOME).
# Masking is per user and needs no sudo; undo with `systemctl --user unmask waybar.service`.
mask_session_units() {
    step "Masking waybar.service (the config starts waybar itself)"
    systemctl --user mask waybar.service
}

build_config_and_pctl() {
    step "Pre-building the config and installing pctl"
    # Building now means the first login does not wait for a full compile.
    (cd "$CONFIG_REPO" && cargo build --package pinnacle-config)
    cargo install --path "$CONFIG_REPO/pctl"

    case ":$PATH:" in
        *":$HOME/.cargo/bin:"*) ;;
        *) echo "Add ~/.cargo/bin to your PATH so \`pctl\` is found." ;;
    esac
}

# Claude Code for the agent menu (Menu → a), user-local in ~/.local, from the official
# installer. Skipped if `claude` is already on PATH.
install_claude_code() {
    step "Installing Claude Code (agent menu)"
    if command -v claude >/dev/null 2>&1; then
        echo "Already installed: $(claude --version 2>/dev/null)"
        return
    fi
    curl -fsSL https://claude.ai/install.sh | bash
    echo "Log in once by running \`claude\` in a terminal (or Menu → a n for an extra profile)."
}

# The folders from settings: notes (captures) and agent (research, slides, hand-offs).
create_folders() {
    step "Creating the notes and agent folders"
    local dir
    for dir in "$(pctl_setting notes.dir)" "$(pctl_setting agent.dir)"; do
        [[ -n "$dir" ]] && mkdir -p "$dir" && echo "$dir"
    done
}

# Reads one setting with the freshly installed pctl.
pctl_setting() {
    PINNACLE_CONFIG_REPO="$CONFIG_REPO" "$HOME/.cargo/bin/pctl" settings get "$1"
}

# Everything after packages and toolchain.
run_common_steps() {
    require_rust
    fetch_and_build_pinnacle
    install_session_files
    install_lua_api
    link_config
    mask_session_units
    build_config_and_pctl
    create_folders
    install_claude_code
    if [[ $WITH_LOCAL_AI == 1 ]]; then
        step "Installing the local model (Ollama)"
        "$CONFIG_REPO/scripts/install-local-ai.sh"
    fi

    step "Done"
    echo "Try it nested first:  $CONFIG_REPO/scripts/nested.sh"
    echo "Then log out and choose \"Pinnacle\" from the session menu on the login screen."
}
