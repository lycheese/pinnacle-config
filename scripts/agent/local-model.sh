#!/usr/bin/env bash
# The local model for the agent menu and capture triage, served by Ollama ([local_ai] url, model).
#
# Claude Code's system prompt alone is larger than Ollama's default context, so the agent runs
# a variant of the model with [local_ai] context tokens of context. This script makes it.
#
#   local-model.sh --check    exit 0 if Ollama answers and the model is pulled
#   local-model.sh --ensure   pull the model if missing and create the variant; print its name
#   local-model.sh --name     print the variant's name
set -euo pipefail
# shellcheck source=scripts/lib/runtime.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/runtime.sh"

url="${PCFG_LOCAL_AI_URL:-http://localhost:11434}"
model="${PCFG_LOCAL_AI_MODEL:-qwen3.5:9b}"
context="${PCFG_LOCAL_AI_CONTEXT:-32768}"
# qwen3.5:9b → qwen3.5-9b-agent-32768
variant="${model//[:\/]/-}-agent-$context"

has_model() {
    curl -sf --max-time 3 "$url/api/tags" | jq -e --arg m "$1" \
        'any(.models[]; .name == $m or .name == ($m + ":latest"))' >/dev/null
}

case "${1:-}" in
    --check)
        has_model "$model"
        ;;
    --name)
        echo "$variant"
        ;;
    --ensure)
        if ! curl -sf --max-time 3 "$url/api/version" >/dev/null; then
            echo "Ollama does not answer at $url. Install it with scripts/install-local-ai.sh," >&2
            echo "or start it with: systemctl start ollama" >&2
            exit 1
        fi
        if ! has_model "$model"; then
            echo "Pulling $model (one-off download) ..." >&2
            curl -sf "$url/api/pull" -d "$(jq -n --arg m "$model" '{model: $m, stream: false}')" \
                >/dev/null || { echo "Could not pull $model" >&2; exit 1; }
        fi
        if ! has_model "$variant"; then
            echo "Creating $variant with $context tokens of context ..." >&2
            curl -sf "$url/api/create" -d "$(jq -n --arg n "$variant" --arg m "$model" \
                --argjson c "$context" '{model: $n, from: $m, parameters: {num_ctx: $c}, stream: false}')" \
                >/dev/null || { echo "Could not create $variant" >&2; exit 1; }
        fi
        echo "$variant"
        ;;
    *)
        echo "usage: local-model.sh --check|--ensure|--name" >&2
        exit 2
        ;;
esac
