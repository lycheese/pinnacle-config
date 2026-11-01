#!/usr/bin/env bash
# Menu → C: capture a thought in a one-line fuzzel prompt and append it to the inbox.
#
#   Enter    save
#   Ctrl+W   attach the window that was focused when the menu opened (press again to drop it)
#   Ctrl+Y   attach the clipboard (press again to drop it)
#   Ctrl+E   continue in the floating Helix capture window
#   Escape   discard
#
# fuzzel has no way to pre-fill its input, so after Ctrl+W / Ctrl+Y the text typed so far moves
# into the prompt and anything typed next is appended to it.
set -euo pipefail
# shellcheck source=scripts/lib/runtime.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/runtime.sh"
# shellcheck source=scripts/capture/lib.sh
source "$REPO/scripts/capture/lib.sh"

capture_snapshot
trap 'rm -rf "$CAPTURE_CTX"' EXIT

text="" win=0 clip=0
while :; do
    flags=""
    ((win)) && flags+=" +window"
    ((clip)) && flags+=" +clipboard"
    prompt="capture${flags:+ [${flags# }]} › ${text:+$text }"
    status=0
    typed="$(fuzzel --dmenu --config="$REPO/scripts/capture/fuzzel.ini" --lines 0 --width 70 \
        --prompt "$prompt" </dev/null)" || status=$?
    if [[ -n "$typed" ]]; then
        text="${text:+$text }$typed"
    fi
    case "$status" in
        0) break ;;
        10) win=$((1 - win)) ;;
        11) clip=$((1 - clip)) ;;
        12)
            # The Helix window takes over; it gets its own copy of the context.
            ctx="$(mktemp -d "${XDG_RUNTIME_DIR:-/tmp}/pinnacle-capture.XXXXXX")"
            cp "$CAPTURE_CTX"/* "$ctx/"
            {
                printf '%s\n' "$text"
                if ((win || clip)); then
                    printf '\n'
                    ((win)) && cat "$ctx/window"
                    ((clip)) && cat "$ctx/clipboard"
                fi
            } >"$ctx/draft"
            CAPTURE_CTX="$ctx" "$REPO/scripts/capture/edit.sh" --draft "$ctx/draft"
            exit 0
            ;;
        *) exit 0 ;;
    esac
done

{
    printf '%s\n' "$text"
    if ((win || clip)) && [[ -s "$CAPTURE_CTX/window" || -s "$CAPTURE_CTX/clipboard" ]]; then
        printf '\n'
        ((win)) && cat "$CAPTURE_CTX/window"
        ((clip)) && cat "$CAPTURE_CTX/clipboard"
    fi
} | capture_save && notify "Captured" "$text"
