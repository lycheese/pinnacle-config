#!/usr/bin/env bash
# Menu → a f: pick a git repo under [agent] repo_roots, describe a feature, and start Claude
# Code there in plan mode (it proposes a plan before changing anything).
#
#   feature.sh [--other]    --other: run on the other backend (Shift+entry)
set -euo pipefail
# shellcheck source=scripts/lib/runtime.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/runtime.sh"
# shellcheck source=scripts/agent/lib.sh
source "$REPO/scripts/agent/lib.sh"

backend="$(pick_backend "${1:-}")"
IFS=: read -r -a roots <<<"${PCFG_AGENT_REPO_ROOTS:-$HOME}"

# Repos up to three levels below each root, most recently changed first.
repo="$(for root in "${roots[@]}"; do
    [[ -d "$root" ]] && find "$root" -maxdepth 4 -name .git -type d -prune -printf '%T@ %h\n' 2>/dev/null
done | sort -rn | cut -d' ' -f2- | awk '!seen[$0]++' | tilde \
    | choose --prompt "repo › " --width 70)" || exit 0
repo="$(untilde <<<"$repo")"
[[ -d "$repo" ]] || exit 0

feature="$(ask_prompt "feature in ${repo##*/}" "$backend")" || exit 0
start_agent "$backend" "$repo" "feature: ${repo##*/}" plan "$feature"
