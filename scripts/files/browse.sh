#!/usr/bin/env bash
# Menu → f f: a Doom Emacs style find-file. Lists one folder at a time in fuzzel, starting in
# your home folder (or the folder given as the first argument).
#
#   Enter           open the file, or go into the folder
#   Alt+Backspace   up to the parent folder (or pick ../)
#   type a path     ~/… or /… jumps there; a name that does not exist is a new file (Helix
#                   creates it on :w), a name ending in / a new folder
#   Ctrl+H          show / hide hidden files
#   Ctrl+F          fuzzy-find any file below this folder (like f F, from here)
#   Ctrl+O          open this folder in the editor (Helix shows its own file picker)
#   Ctrl+T          open a terminal in this folder
#   Escape          cancel
set -euo pipefail
# shellcheck source=scripts/lib/runtime.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/runtime.sh"

dir="$(realpath "${1:-$HOME}")"
hidden=0

# Folders first (with a trailing /), then files, each sorted by name; ../ on top.
# Each line carries a fuzzel icon: "\0icon\x1f<name>".
list() {
    [[ "$dir" != / ]] && printf '../\0icon\x1fgo-up\n'
    local -a filter=()
    ((hidden)) || filter=(-not -name '.*')
    find "$dir" -mindepth 1 -maxdepth 1 "${filter[@]}" -xtype d -printf '%f/\n' 2>/dev/null \
        | LC_ALL=C sort -f | sed 's/$/\x00icon\x1ffolder/'
    find "$dir" -mindepth 1 -maxdepth 1 "${filter[@]}" -not -xtype d -printf '%f\n' 2>/dev/null \
        | LC_ALL=C sort -f | sed 's/$/\x00icon\x1ftext-x-generic/'
}

open_file() {
    mkdir -p "$(dirname "$1")"
    exec "$REPO/scripts/files/open.sh" --file "$1"
}

while :; do
    shown="$(tilde <<<"$dir")"
    [[ "$shown" == / ]] || shown+=/
    flags=""
    ((hidden)) && flags=" [+hidden]"
    status=0
    picked="$(list | fuzzel --dmenu --config="$REPO/scripts/files/fuzzel.ini" --width 70 \
        --prompt "$shown$flags › ")" || status=$?

    case "$status" in
        0) ;;
        10) dir="$(dirname "$dir")"; continue ;;
        11) hidden=$((1 - hidden)); continue ;;
        12)
            file="$("$REPO/scripts/files/pick.sh" --root "$dir")" || continue
            open_file "$file"
            ;;
        13) open_in_editor "$dir"; exit 0 ;;
        14)
            (cd "$dir" && setsid -f "${PCFG_APPS_TERMINAL:-alacritty}" >/dev/null 2>&1)
            exit 0
            ;;
        *) exit 0 ;;
    esac
    [[ -n "$picked" ]] || continue

    case "$picked" in
        ../) dir="$(dirname "$dir")"; continue ;;
        # A typed ~ is text from fuzzel, so match it literally.
        \~ | \~/) target="$HOME" ;;
        \~/*) target="$HOME/${picked#\~/}" ;;
        /*) target="$picked" ;;
        *) target="${dir%/}/$picked" ;;
    esac

    if [[ -d "$target" ]]; then
        dir="$(realpath "$target")"
    elif [[ "$picked" == */ ]]; then
        mkdir -p "$target" && notify "Created folder" "$(tilde <<<"$target")"
        dir="$(realpath "$target")"
    else
        open_file "$target"
    fi
done
