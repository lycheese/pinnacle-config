#!/usr/bin/env bash
# Opens a file in the editor and remembers it in $STATE_DIR/recent-files.
#
#   open.sh               Menu → f F: fuzzy-find a file under [files] search_roots
#   open.sh --recent      Menu → f r: pick a recently opened file
#   open.sh --file <path> open that file (used by the find-file browser, f f)
set -euo pipefail
# shellcheck source=scripts/lib/runtime.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/runtime.sh"

recent="$STATE_DIR/recent-files"
touch "$recent"

if [[ "${1:-}" == --file ]]; then
    file="$2"
elif [[ "${1:-}" == --recent ]]; then
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
