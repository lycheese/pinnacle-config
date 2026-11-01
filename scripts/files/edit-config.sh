#!/usr/bin/env bash
# Menu → f c: open this config repo in the editor. Menu → f s (--local): open local.toml,
# creating it with a short header first.
set -euo pipefail
# shellcheck source=scripts/lib/runtime.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/runtime.sh"

if [[ "${1:-}" == --local ]]; then
    local_toml="$REPO/local.toml"
    if [[ ! -e "$local_toml" ]]; then
        cat >"$local_toml" <<'EOF'
# Personal overrides for settings.toml (gitignored). Set only the keys you change, in the same
# [sections]. Reload the config (Super+Ctrl+R) to apply.
EOF
    fi
    open_in_editor "$local_toml"
else
    open_in_editor "$REPO"
fi
