# shellcheck shell=bash
# scripts/agent/: backends, thread folders, local model names.

load() {
    # shellcheck source=scripts/lib/runtime.sh
    source "$REPO/scripts/lib/runtime.sh"
    # shellcheck source=scripts/agent/lib.sh
    source "$REPO/scripts/agent/lib.sh"
}

test_backend_defaults_to_settings_then_follows_toggle() {
    load
    assert_eq "$(agent_backend)" "claude"
    assert_eq "$(other_backend)" "local"
    echo local >"$BACKEND_FILE"
    assert_eq "$(agent_backend)" "local"
    assert_eq "$(other_backend)" "claude"
}

test_pick_backend_other_flag() {
    load
    assert_eq "$(pick_backend)" "claude"
    assert_eq "$(pick_backend --other)" "local"
}

test_backend_script_toggles_state() {
    load
    PATH="$SANDBOX:$PATH"
    printf '#!/bin/sh\n' >"$SANDBOX/notify-send" && chmod +x "$SANDBOX/notify-send"
    "$REPO/scripts/agent/backend.sh" --toggle
    assert_eq "$("$REPO/scripts/agent/backend.sh")" "local"
    "$REPO/scripts/agent/backend.sh" --toggle
    assert_eq "$("$REPO/scripts/agent/backend.sh")" "claude"
}

test_thread_dirs_are_dated_and_never_reused() {
    load
    local a b
    a="$(new_thread_dir research "What is Wayland?")"
    b="$(new_thread_dir research "What is Wayland?")"
    assert_eq "$a" "$HOME/agent/research/$(date +%Y-%m-%d)-what-is-wayland"
    [[ -d "$a" && -d "$b" && "$a" != "$b" ]] || fail "second folder should differ: $b"
}

test_local_model_variant_name() {
    load
    assert_eq "$("$REPO/scripts/agent/local-model.sh" --name)" "qwen3.5-9b-agent-32768"
}
