#!/usr/bin/env bash
# Menu → a r (research) / a g (slides): ask for a topic, make a dated folder for it under
# [agent] dir, and start Claude Code there with edits allowed in that folder.
#
#   thread.sh research|slides [--other]    --other: run on the other backend (Shift+entry)
set -euo pipefail
# shellcheck source=scripts/lib/runtime.sh
source "$(dirname "${BASH_SOURCE[0]}")/../lib/runtime.sh"
# shellcheck source=scripts/agent/lib.sh
source "$REPO/scripts/agent/lib.sh"

kind="$1"
backend="$(pick_backend "${2:-}")"

case "$kind" in
    research)
        topic="$(ask_prompt "research" "$backend")" || exit 0
        prompt="Research this and write the findings to report.md in this folder, with a short
summary at the top and links to your sources:

$topic"
        ;;
    slides)
        topic="$(ask_prompt "slides about" "$backend")" || exit 0
        prompt="Make an HTML slide deck in this folder as one self-contained slides.html (inline
CSS and JavaScript, arrow keys to move between slides, readable at 1920x1080). Draw graphics
as inline SVG. Topic and notes:

$topic"
        ;;
    *)
        echo "usage: thread.sh research|slides [--other]" >&2
        exit 2
        ;;
esac

dir="$(new_thread_dir "$kind" "$topic")"
start_agent "$backend" "$dir" "$kind: $topic" acceptEdits "$prompt"
