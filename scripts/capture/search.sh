#!/usr/bin/env bash
# Menu → n s: fuzzy-search the captures in the inbox (date, tags, first line) and open the
# picked one in the editor at its heading. Newest first.
# Menu → n i (--open): open the inbox at its last line.
set -euo pipefail
# shellcheck source=scripts/lib/runtime.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/runtime.sh"
# shellcheck source=scripts/capture/lib.sh
source "$REPO/scripts/capture/lib.sh"

if [[ "${1:-}" == --open ]]; then
    open_in_editor "$INBOX:$(wc -l <"$INBOX")"
    exit 0
fi

# "<line>\t<date and tags> · <first line of the text>" per capture.
entries="$(awk '
    /^## / { if (line) print line "\t" head " · " first; line = NR; head = substr($0, 4); first = ""; next }
    line && first == "" && NF { first = $0 }
    END { if (line) print line "\t" head " · " first }
' "$INBOX" | tac)"
[[ -n "$entries" ]] || { notify "No captures yet" "$INBOX"; exit 0; }

index="$(cut -f2- <<<"$entries" | choose --index --width 90 --prompt "captures › ")" || exit 0
line="$(sed -n "$((index + 1))p" <<<"$entries" | cut -f1)"
open_in_editor "$INBOX:$line"
