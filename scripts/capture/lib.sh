# shellcheck shell=bash
# Shared by the capture scripts. Source after scripts/lib/runtime.sh.
#
# Captures go to [notes] dir/inbox.md, newest last, one entry per capture:
#
#   ## 2026-11-01 19:05 #idea #pinnacle
#
#   The text you typed.
#
#   - window: firefox — Page title
#   - clipboard:
#   ~~~~
#   what was on the clipboard
#   ~~~~

NOTES_DIR="${PCFG_NOTES_DIR:-$HOME/notes}"
INBOX="$NOTES_DIR/inbox.md"
mkdir -p "$NOTES_DIR"
[[ -e "$INBOX" ]] || printf '# Inbox\n' >"$INBOX"

# Records the focused window and the clipboard into $CAPTURE_CTX (a new temp dir, exported) so
# they can be attached later. Run it before any capture window opens, or the focused window
# would be the capture window itself.
capture_snapshot() {
    CAPTURE_CTX="$(mktemp -d "${XDG_RUNTIME_DIR:-/tmp}/pinnacle-capture.XXXXXX")"
    export CAPTURE_CTX
    "$PCTL" focused 2>/dev/null \
        | jq -r 'if . == null then empty else "- window: \(.app_id) — \(.title)" end' \
            >"$CAPTURE_CTX/window" 2>/dev/null || true
    : >"$CAPTURE_CTX/clipboard"
    if wl-paste --list-types 2>/dev/null | grep -q '^text/'; then
        {
            printf -- '- clipboard:\n~~~~\n'
            wl-paste --no-newline 2>/dev/null | head -c 4000
            printf '\n~~~~\n'
        } >"$CAPTURE_CTX/clipboard"
    fi
}

# Prints the #tags in a capture body (outside ~~~~ blocks), space separated.
capture_tags() {
    awk '/^~~~~/ { fence = !fence; next } !fence' \
        | { grep -oE '(^|[[:space:]])#[[:alnum:]_-]+' || true; } \
        | sed 's/^[[:space:]]*//' | awk '!seen[$0]++' | paste -sd ' ' -
}

# Appends a capture to the inbox. The body comes from stdin; empty bodies are dropped.
capture_save() {
    local body tags
    body="$(cat)"
    [[ -n "${body//[[:space:]]/}" ]] || return 1
    tags="$(printf '%s\n' "$body" | capture_tags)"
    printf '\n## %s%s\n\n%s\n' "$(date '+%Y-%m-%d %H:%M')" "${tags:+ $tags}" "$body" >>"$INBOX"
}
