#!/usr/bin/env bash
# Menu → a b: switch the agent menu between Claude Code with your account ("claude") and Claude
# Code against a local model in Ollama ("local"). The menu breadcrumb shows the active one.
#
#   backend.sh --toggle   switch and announce it
#   backend.sh            print the active backend
set -euo pipefail
# shellcheck source=scripts/lib/runtime.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/runtime.sh"
# shellcheck source=scripts/agent/lib.sh
source "$REPO/scripts/agent/lib.sh"

if [[ "${1:-}" != --toggle ]]; then
    agent_backend
    exit 0
fi

next="$(other_backend)"
printf '%s\n' "$next" >"$BACKEND_FILE"
if [[ "$next" == local ]]; then
    if "$REPO/scripts/agent/local-model.sh" --check; then
        notify "Agent backend: local" "${PCFG_LOCAL_AI_MODEL:-qwen3.5:9b} via Ollama"
    else
        notify "Agent backend: local" "Ollama or the model is missing; see scripts/install-local-ai.sh"
    fi
else
    notify "Agent backend: Claude"
fi
