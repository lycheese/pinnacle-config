# shellcheck shell=bash
# scripts/capture/lib.sh: inbox entries and #tags.

load() {
    # shellcheck source=scripts/lib/runtime.sh
    source "$REPO/scripts/lib/runtime.sh"
    # shellcheck source=scripts/capture/lib.sh
    source "$REPO/scripts/capture/lib.sh"
}

test_inbox_is_created_with_a_title() {
    load
    assert_eq "$(cat "$HOME/notes/inbox.md")" "# Inbox"
}

test_tags_are_found_once_and_not_in_fenced_context() {
    load
    local body=$'Idea #pinnacle and #idea, again #pinnacle\nissue#5 is not a tag\n~~~~\n#clipboard\n~~~~'
    assert_eq "$(capture_tags <<<"$body")" "#pinnacle #idea"
}

test_save_appends_a_dated_entry_with_tags() {
    load
    printf 'Buy #milk\n\n- window: kitty — shell\n' | capture_save
    local inbox
    inbox="$(cat "$INBOX")"
    assert_contains "$inbox" "## $(date +%Y-%m-%d) "
    assert_contains "$inbox" " #milk"$'\n\nBuy #milk\n\n- window: kitty — shell'
}

test_save_drops_blank_captures() {
    load
    assert_status 1 capture_save <<<$'  \n\t'
    assert_eq "$(cat "$INBOX")" "# Inbox"
}

test_search_lists_newest_capture_first() {
    load
    printf 'first\n' | capture_save
    printf 'second #b\n' | capture_save
    # The listing search.sh builds: "<line>\t<heading> · <first line>".
    local listing
    listing="$(sed -n '/^entries="$(awk/,/^. "$INBOX" | tac)"$/p' "$REPO/scripts/capture/search.sh")"
    [[ -n "$listing" ]] || fail "listing code not found in search.sh"
    eval "$listing"
    # shellcheck disable=SC2154 # set by the eval
    assert_contains "$(head -n 1 <<<"$entries")" "second #b"
    assert_contains "$(tail -n 1 <<<"$entries")" "· first"
}
