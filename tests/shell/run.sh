#!/usr/bin/env bash
# Runs the shell tests: every `test_*` function in tests/shell/test_*.sh, each in its own
# subshell with a throwaway $HOME, state folder and notes folder. No compositor needed.
#
#   tests/shell/run.sh [name-filter]     PCTL=<path> picks the pctl binary (default: PATH)
#
# Writing a test: a function named test_<what>, using the assert_* helpers below; a failing
# assert prints the reason and fails the test. Source the script under test inside the test.
set -uo pipefail

TESTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$TESTS_DIR/../.." && pwd)"
export REPO
filter="${1:-}"

fail() {
    printf '    %s\n' "$@" >&2
    exit 1
}
assert_eq() {
    [[ "$1" == "$2" ]] || fail "expected: $(printf '%q' "$2")" "     got: $(printf '%q' "$1")" ${3:+"($3)"}
}
assert_contains() {
    [[ "$1" == *"$2"* ]] || fail "expected to contain: $2" "in: $1"
}
assert_not_contains() {
    [[ "$1" != *"$2"* ]] || fail "expected not to contain: $2" "in: $1"
}
assert_file() {
    [[ -f "$1" ]] || fail "missing file: $1"
}
assert_status() {
    local want="$1"
    shift
    local got=0
    "$@" >/dev/null 2>&1 || got=$?
    [[ "$got" == "$want" ]] || fail "expected exit $want, got $got: $*"
}
export -f fail assert_eq assert_contains assert_not_contains assert_file assert_status

passed=0 failed=0
for file in "$TESTS_DIR"/test_*.sh; do
    tests="$(bash -c "source '$file'; declare -F" | awk '$3 ~ /^test_/ { print $3 }')"
    for t in $tests; do
        [[ -z "$filter" || "$t" == *"$filter"* ]] || continue
        sandbox="$(mktemp -d)"
        # Settings come from a copy of settings.toml only, so your local.toml cannot change results.
        mkdir -p "$sandbox/home" "$sandbox/run" "$sandbox/settings"
        cp "$REPO/settings.toml" "$sandbox/settings/"
        # Not inside `if`: bash ignores `set -e` there, so a failing command would not fail the test.
        (
            export HOME="$sandbox/home" XDG_STATE_HOME="$sandbox/state" \
                XDG_RUNTIME_DIR="$sandbox/run" PINNACLE_CONFIG_REPO="$sandbox/settings" \
                SANDBOX="$sandbox"
            cd "$sandbox" || exit 1
            # shellcheck source=/dev/null
            source "$file"
            set -e
            "$t"
        )
        status=$?
        if ((status == 0)); then
            passed=$((passed + 1))
            printf 'ok    %s  %s\n' "${file##*/}" "$t"
        else
            failed=$((failed + 1))
            printf 'FAIL  %s  %s\n' "${file##*/}" "$t"
        fi
        rm -rf "$sandbox"
    done
done

printf '\n%d passed, %d failed\n' "$passed" "$failed"
((failed == 0))
