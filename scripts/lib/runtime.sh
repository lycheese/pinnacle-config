# shellcheck shell=bash
# Helpers for scripts the config runs at runtime (menu actions, capture, agent).
# Source it first thing: `source "$(dirname "${BASH_SOURCE[0]}")/lib/runtime.sh"`.
#
# It loads settings.toml + local.toml as PCFG_* variables (see `pctl settings --env`).

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
STATE_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/pinnacle-config"
mkdir -p "$STATE_DIR"

# pctl lives in ~/.cargo/bin, which may be missing from PATH when started by the compositor.
PCTL="$(command -v pctl || echo "$HOME/.cargo/bin/pctl")"
export PINNACLE_CONFIG_REPO="$REPO"
eval "$("$PCTL" settings --env)"

# fd is called fdfind on Debian/Ubuntu.
fd_bin() { command -v fd || command -v fdfind; }

notify() { notify-send --app-name=pinnacle-config "$@" 2>/dev/null || true; }

# Runs a command in a new terminal window: in_terminal <app-id> <title> <command...>
# The terminal command comes from [apps] terminal_run, where %c is the app ID and %t the title.
in_terminal() {
    local class="$1" title="$2"
    shift 2
    local run="${PCFG_APPS_TERMINAL_RUN:-alacritty --class %c --title %t -e}"
    local -a words
    read -r -a words <<<"$run"
    local -a cmd=()
    local w
    for w in "${words[@]}"; do
        case "$w" in
            %c) cmd+=("$class") ;;
            %t) cmd+=("$title") ;;
            *) cmd+=("$w") ;;
        esac
    done
    setsid -f "${cmd[@]}" "$@" >/dev/null 2>&1
}

# Opens files or folders in the editor from [apps] editor, in a terminal.
open_in_editor() {
    local -a editor
    read -r -a editor <<<"${PCFG_APPS_EDITOR:-hx}"
    in_terminal pinnacle-editor "${1##*/}" "${editor[@]}" "$@"
}

# Asks for one line of text in fuzzel; fails on Escape or empty input. Extra args go to fuzzel.
ask_line() {
    local line
    line="$(fuzzel --dmenu --lines 0 --width 70 "$@" </dev/null)" || return 1
    [[ -n "${line//[[:space:]]/}" ]] || return 1
    printf '%s\n' "$line"
}

# Turns text into a short folder name: "Fix the login bug!" → fix-the-login-bug
slugify() {
    tr '[:upper:]' '[:lower:]' <<<"$1" | sed -E 's/[^a-z0-9]+/-/g; s/^-+//' | cut -c1-40 | sed -E 's/-+$//'
}

# One-line fuzzel menu over stdin; prints the chosen line. Extra args go to fuzzel.
choose() { fuzzel --dmenu "$@"; }

# Shows paths as ~/… for menus, and turns them back into full paths.
tilde() { sed "s#^$HOME/#~/#"; }
untilde() { sed "s#^~/#$HOME/#"; }
