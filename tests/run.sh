#!/usr/bin/env bash
# Runs every test: Rust unit tests, the shell tests (with a freshly built pctl) and the Python
# tests. No running compositor needed. Exits non-zero if any suite fails; all suites always run.
#
#   tests/run.sh
set -uo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1

failed=()
suite() {
    local name="$1"
    shift
    printf '\n\033[1;34m==> %s\033[0m\n' "$name"
    "$@" || failed+=("$name")
}

suite "Rust (cargo test)" cargo test --workspace
suite "Build pctl" cargo build --package pctl
suite "Shell" env PCTL="$PWD/target/debug/pctl" tests/shell/run.sh
suite "Python" env PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s tests/python

echo
if ((${#failed[@]})); then
    printf '\033[1;31mFailed: %s\033[0m\n' "${failed[*]}"
    exit 1
fi
printf '\033[1;32mAll tests passed\033[0m\n'
