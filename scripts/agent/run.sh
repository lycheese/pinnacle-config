#!/usr/bin/env bash
# Runs Claude Code inside the agent terminal (started by start_agent in scripts/agent/lib.sh).
#
#   run.sh <claude|local> <profile> <folder> [claude args...]
#
# "local" points Claude Code at Ollama's Anthropic-compatible API ([local_ai] url and model).
# A profile other than "default" is a CLAUDE_CONFIG_DIR under [agent] profiles_dir.
set -uo pipefail
# shellcheck source=scripts/lib/runtime.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/runtime.sh"

backend="$1" profile="$2" dir="$3"
shift 3

if [[ "$profile" != default ]]; then
    export CLAUDE_CONFIG_DIR="${PCFG_AGENT_PROFILES_DIR:-$HOME/.config/claude-profiles}/$profile"
fi

args=()
if [[ "$backend" == local ]]; then
    export ANTHROPIC_BASE_URL="${PCFG_LOCAL_AI_URL:-http://localhost:11434}"
    export ANTHROPIC_AUTH_TOKEN=ollama
    export ANTHROPIC_API_KEY=""
    if ! model="$("$REPO/scripts/agent/local-model.sh" --ensure)"; then
        printf '\nThe local model is not available. Press Enter to close.'
        read -r _
        exit 1
    fi
    args+=(--model "$model")
fi

read -r -a claude <<<"${PCFG_AGENT_COMMAND:-claude}"
cd "$dir" || exit 1
"${claude[@]}" "${args[@]}" "$@"
status=$?
if ((status != 0)); then
    printf '\n%s exited with status %d. Press Enter to close.' "${claude[0]}" "$status"
    read -r _
fi
