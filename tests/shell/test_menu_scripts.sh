# shellcheck shell=bash
# Every script the leader menu runs exists and is executable.

test_menu_scripts_exist_and_are_executable() {
    local paths
    paths="$(grep -oE 'script\([^;]*"[a-z-]+/[a-z-]+\.sh"' "$REPO/config/src/leader/tree.rs" \
        | grep -oE '"[a-z-]+/[a-z-]+\.sh"' | tr -d '"' | sort -u)"
    [[ -n "$paths" ]] || fail "no script() entries found in tree.rs"
    local p
    for p in $paths; do
        [[ -x "$REPO/scripts/$p" ]] || fail "scripts/$p is missing or not executable"
    done
}

test_all_scripts_parse() {
    local f
    while IFS= read -r -d '' f; do
        bash -n "$f" || fail "syntax error in $f"
    done < <(find "$REPO/scripts" "$REPO/tests" -name '*.sh' -print0)
}
