#!/usr/bin/env bash
# Claude Code profiles: separate CLAUDE_CONFIG_DIRs, each with its own login and settings.
# "default" is ~/.claude (shared with the Claude desktop app); others live in
# [agent] profiles_dir/<name>. The active one is used by every agent entry.
#
#   profile.sh --switch   Menu → a p: pick the active profile
#   profile.sh --new      Menu → a n: name a new profile, make it active and log in
set -euo pipefail
# shellcheck source=scripts/lib/runtime.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/runtime.sh"
# shellcheck source=scripts/agent/lib.sh
source "$REPO/scripts/agent/lib.sh"

profiles_dir="${PCFG_AGENT_PROFILES_DIR:-$HOME/.config/claude-profiles}"
mkdir -p "$profiles_dir"

case "${1:-}" in
    --switch)
        current="$(agent_profile)"
        picked="$({
            echo default
            find "$profiles_dir" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort
        } | sed "s/^$current\$/$current (active)/" \
            | choose --prompt "claude profile › ")" || exit 0
        picked="${picked% (active)}"
        printf '%s\n' "$picked" >"$PROFILE_FILE"
        notify "Claude profile: $picked"
        ;;
    --new)
        name="$(ask_line --prompt "new claude profile name › ")" || exit 0
        name="$(slugify "$name")"
        [[ -n "$name" && "$name" != default ]] || exit 0
        mkdir -p "$profiles_dir/$name"
        printf '%s\n' "$name" >"$PROFILE_FILE"
        notify "Claude profile: $name" "Log in with /login in the window that opens"
        mkdir -p "$AGENT_DIR"
        in_terminal pinnacle-agent "claude profile: $name" \
            "$REPO/scripts/agent/run.sh" claude "$name" "$AGENT_DIR"
        ;;
    *)
        echo "usage: profile.sh --switch|--new" >&2
        exit 2
        ;;
esac
