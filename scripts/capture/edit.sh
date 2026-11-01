#!/usr/bin/env bash
# Menu → n c (or Ctrl+E in the capture prompt): write a capture in a floating Helix window.
# :wq appends it to the inbox, :q! discards it. Keys inside: see scripts/capture/helix.toml.
#
#   edit.sh [--draft <file>]   open the window, starting from <file>'s text
#   edit.sh --inside <file>    (internal) runs in the window: Helix, then save
set -euo pipefail
# shellcheck source=scripts/lib/runtime.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/runtime.sh"
# shellcheck source=scripts/capture/lib.sh
source "$REPO/scripts/capture/lib.sh"

if [[ "${1:-}" == --inside ]]; then
    draft="$2"
    trap 'rm -rf "$CAPTURE_CTX"' EXIT
    # Saved means the file was written after this marker, even with unchanged text.
    touch "$CAPTURE_CTX/opened"
    "${PCFG_APPS_EDITOR:-hx}" -c "$REPO/scripts/capture/helix.toml" "$draft:1" || true
    if [[ "$draft" -nt "$CAPTURE_CTX/opened" ]]; then
        capture_save <"$draft" && notify "Captured" "$(head -n 1 "$draft")"
    fi
    exit 0
fi

# Context comes from the caller (quick.sh) or is taken now, before the window opens.
[[ -n "${CAPTURE_CTX:-}" && -d "$CAPTURE_CTX" ]] || capture_snapshot
draft="$CAPTURE_CTX/capture.md"
if [[ "${1:-}" == --draft && -f "${2:-}" ]]; then
    [[ "$2" == "$draft" ]] || cp "$2" "$draft"
else
    : >"$draft"
fi
in_terminal pinnacle-capture "Capture — :wq saves, :q! discards" \
    "$REPO/scripts/capture/edit.sh" --inside "$draft"
