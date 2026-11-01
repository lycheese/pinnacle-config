#!/usr/bin/env bash
# Installs the local model for the agent menu's "local" backend and for capture triage:
#
#   1. Ollama, from its official installer (system-wide, asks for sudo; sets up the
#      ollama.service and uses an NVIDIA/AMD GPU when it finds one)
#   2. the model from [local_ai] model (default qwen3.5:9b, ~6 GB download)
#   3. a variant with [local_ai] context tokens of context for Claude Code
#
# Safe to re-run. Also run by the install scripts with --with-local-ai.
set -euo pipefail
REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if command -v ollama >/dev/null 2>&1; then
    echo "Ollama is installed: $(ollama --version 2>/dev/null | head -n 1)"
else
    echo "Installing Ollama (https://ollama.com/install.sh, needs sudo)"
    curl -fsSL https://ollama.com/install.sh | sh
fi

# Wait for the service to answer (it starts right after install).
url="$(PINNACLE_CONFIG_REPO="$REPO" "${PCTL:-$HOME/.cargo/bin/pctl}" settings get local_ai.url 2>/dev/null \
    || echo http://localhost:11434)"
for _ in $(seq 30); do
    curl -sf --max-time 2 "$url/api/version" >/dev/null && break
    sleep 1
done

model="$("$REPO/scripts/agent/local-model.sh" --ensure)"
echo "Ready: $model. Switch the agent menu to it with Super+Space a b."
