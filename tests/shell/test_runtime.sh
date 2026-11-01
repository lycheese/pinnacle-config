# shellcheck shell=bash
# scripts/lib/runtime.sh: settings, names, paths and terminal commands.

load() {
    # shellcheck source=scripts/lib/runtime.sh
    source "$REPO/scripts/lib/runtime.sh"
}

test_settings_are_loaded_as_env() {
    load
    assert_eq "$PCFG_APPS_EDITOR" "hx"
    assert_eq "$PCFG_NOTES_DIR" "$HOME/notes" "~ is expanded"
}

test_slugify() {
    load
    assert_eq "$(slugify "Fix the Login bug!")" "fix-the-login-bug"
    assert_eq "$(slugify "  --Leading and trailing--  ")" "leading-and-trailing"
    assert_eq "$(slugify "a very long topic name that goes on and on and on forever")" \
        "a-very-long-topic-name-that-goes-on-and"
}

test_tilde_round_trip() {
    load
    assert_eq "$(echo "$HOME/a/b" | tilde)" "~/a/b"
    assert_eq "$(echo "~/a/b" | untilde)" "$HOME/a/b"
    assert_eq "$(echo "/etc/$HOME" | tilde)" "/etc/$HOME" "only a leading home folder"
}

test_in_terminal_fills_in_class_and_title() {
    load
    local log="$SANDBOX/args"
    printf '#!/bin/sh\nfor a in "$@"; do echo "$a"; done >"%s"\n' "$log" >"$SANDBOX/term"
    chmod +x "$SANDBOX/term"
    PCFG_APPS_TERMINAL_RUN="$SANDBOX/term --class %c --title %t -e"
    in_terminal my-class "A title" echo "two words"
    for _ in $(seq 50); do [[ -s "$log" ]] && break; sleep 0.1; done
    assert_eq "$(cat "$log")" $'--class\nmy-class\n--title\nA title\n-e\necho\ntwo words'
}
