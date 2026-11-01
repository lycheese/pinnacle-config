#!/usr/bin/env bash
# Menu → f f / f r: pick a file (or a recent one with --recent) and open it in the editor.
# Opened files are remembered in $STATE_DIR/recent-files for --recent.
set -euo pipefail
# shellcheck source=scripts/lib/runtime.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/runtime.sh"

recent="$STATE_DIR/recent-files"
touch "$recent"

if [[ "${1:-}" == --recent ]]; then
    # Newest first, no duplicates, only files that still exist.
    file="$(tac "$recent" | awk '!seen[$0]++' | while IFS= read -r f; do
        [[ -f "$f" ]] && printf '%s\n' "$f"
    done | tilde | choose --prompt "recent › " | untilde)" || exit 0
else
    file="$("$REPO/scripts/files/pick.sh")" || exit 0
fi
[[ -n "$file" ]] || exit 0

printf '%s\n' "$file" >>"$recent"
# Keep the history short.
tail -n 500 "$recent" >"$recent.tmp" && mv "$recent.tmp" "$recent"

open_in_editor "$file"
