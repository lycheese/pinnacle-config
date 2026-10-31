#!/usr/bin/env bash
# Run Pinnacle as a window inside your current GNOME session, using this repo's config.
# Mod is Alt in this mode (Super stays with GNOME). Close the window or press Alt+Ctrl+Shift+Q to quit.
#
# Usage: scripts/nested.sh [path/to/pinnacle-binary]
#   e.g. scripts/nested.sh ~/src/pinnacle/target/debug/pinnacle   to test your own compositor build

set -euo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bin="${1:-pinnacle}"

exec "$bin" --config-dir "$repo"
