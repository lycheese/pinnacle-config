#!/usr/bin/env bash
# Prints one file picked with fuzzel from [files] search_roots (hidden and .gitignored files
# are skipped). Exits non-zero if nothing was picked. Extra args go to fuzzel.
set -euo pipefail
# shellcheck source=scripts/lib/runtime.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/runtime.sh"

fd="$(fd_bin)" || { notify "fd is not installed"; exit 1; }
IFS=: read -r -a roots <<<"${PCFG_FILES_SEARCH_ROOTS:-$HOME}"

picked="$("$fd" --type f --absolute-path --exclude .git . "${roots[@]}" 2>/dev/null \
    | tilde | choose --prompt "file › " "$@")" || exit 1
[[ -n "$picked" ]] || exit 1
printf '%s\n' "$picked" | untilde
