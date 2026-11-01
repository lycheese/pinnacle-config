#!/usr/bin/env bash
# Menu → n t: sort the capture inbox into todo.md / archive.md with the local model (Ollama,
# [local_ai] model; captures never leave the machine). Unclear ones stay in the inbox. The inbox
# is backed up to ~/.local/state/pinnacle-config/inbox-backups first. See triage.py.
set -euo pipefail
# shellcheck source=scripts/lib/runtime.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/runtime.sh"
# shellcheck source=scripts/capture/lib.sh
source "$REPO/scripts/capture/lib.sh"

if ! "$REPO/scripts/agent/local-model.sh" --check; then
    notify "Triage needs the local model" "Ollama with ${PCFG_LOCAL_AI_MODEL:-qwen3.5:9b}; see scripts/install-local-ai.sh"
    exit 1
fi

notify "Triaging the inbox" "with ${PCFG_LOCAL_AI_MODEL:-qwen3.5:9b} …"
status=0
summary="$(INBOX="$INBOX" NOTES_DIR="$NOTES_DIR" BACKUP_DIR="$STATE_DIR/inbox-backups" \
    OLLAMA_URL="${PCFG_LOCAL_AI_URL:-http://localhost:11434}" \
    MODEL="${PCFG_LOCAL_AI_MODEL:-qwen3.5:9b}" \
    python3 -I "$REPO/scripts/capture/triage.py")" || status=$?
printf '%s %s\n' "$(date '+%F %T')" "$summary" >>"$STATE_DIR/triage.log"
notify "Inbox triage" "$summary"
exit "$status"
