#!/usr/bin/env bash
# Menu → a w / a s / a a / a c: hand something to the agent, then ask about it.
#
#   handoff.sh --window      screenshot of the focused window
#   handoff.sh --region      screenshot of a region you drag (slurp)
#   handoff.sh --file        a file you pick (the agent may read its folder)
#   handoff.sh --clipboard   the clipboard (text or image)
#   ... [--other]            run on the other backend (Shift+entry)
#
# The capture is taken first, so the window is the one focused when the menu opened. Files land
# in a dated folder under [agent] dir/handoff, where Claude Code starts with edits allowed.
set -euo pipefail
# shellcheck source=scripts/lib/runtime.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/runtime.sh"
# shellcheck source=scripts/agent/lib.sh
source "$REPO/scripts/agent/lib.sh"

what="${1:-}"
backend="$(pick_backend "${2:-}")"
tmp="$(mktemp -d "${XDG_RUNTIME_DIR:-/tmp}/pinnacle-handoff.XXXXXX")"
trap 'rm -rf "$tmp"' EXIT
extra_args=()

case "$what" in
    --window)
        geometry="$("$PCTL" focused 2>/dev/null \
            | jq -r 'if . == null then empty else "\(.loc.x),\(.loc.y) \(.size.w)x\(.size.h)" end')"
        [[ -n "$geometry" ]] || { notify "No focused window"; exit 0; }
        grim -g "$geometry" "$tmp/window.png"
        attached="the screenshot ./window.png"
        label="about this window"
        ;;
    --region)
        geometry="$(slurp)" || exit 0
        grim -g "$geometry" "$tmp/region.png"
        attached="the screenshot ./region.png"
        label="about this region"
        ;;
    --file)
        file="$("$REPO/scripts/files/pick.sh" --prompt "hand off › ")" || exit 0
        attached="the file $file"
        extra_args+=(--add-dir "$(dirname "$file")")
        label="about ${file##*/}"
        ;;
    --clipboard)
        types="$(wl-paste --list-types 2>/dev/null)" || { notify "The clipboard is empty"; exit 0; }
        if grep -q '^image/png' <<<"$types"; then
            wl-paste --type image/png >"$tmp/clipboard.png"
            attached="the image ./clipboard.png"
        else
            wl-paste --no-newline >"$tmp/clipboard.txt"
            attached="the text in ./clipboard.txt"
        fi
        label="about the clipboard"
        ;;
    *)
        echo "usage: handoff.sh --window|--region|--file|--clipboard [--other]" >&2
        exit 2
        ;;
esac

question="$(ask_prompt "$label" "$backend")" || exit 0
dir="$(new_thread_dir handoff "$question")"
cp -r "$tmp/." "$dir/"
start_agent "$backend" "$dir" "agent: $question" acceptEdits \
    "$question

Look at $attached first." "${extra_args[@]}"
