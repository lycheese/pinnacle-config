# shellcheck shell=bash
# Shared by the agent scripts. Source after scripts/lib/runtime.sh.
#
# Every menu entry ends in start_agent, which opens a terminal running Claude Code in a folder
# with a first prompt, on the active backend and profile.

AGENT_DIR="${PCFG_AGENT_DIR:-$HOME/agent}"
PROFILE_FILE="$STATE_DIR/agent-profile"
BACKEND_FILE="$STATE_DIR/agent-backend"

# The active Claude profile name ("default" = ~/.claude).
agent_profile() { cat "$PROFILE_FILE" 2>/dev/null || echo default; }

# The active backend: what Menu → a b last chose, else [agent] backend.
agent_backend() { cat "$BACKEND_FILE" 2>/dev/null || echo "${PCFG_AGENT_BACKEND:-claude}"; }

# The other backend, for Shift+entry.
other_backend() { [[ "$(agent_backend)" == local ]] && echo claude || echo local; }

# A new dated folder under $AGENT_DIR/<kind>/ named after the prompt; prints its path.
new_thread_dir() {
    local kind="$1" prompt="$2" dir
    dir="$AGENT_DIR/$kind/$(date +%Y-%m-%d)-$(slugify "$prompt")"
    [[ -e "$dir" ]] && dir+="-$(date +%H%M%S)"
    mkdir -p "$dir"
    printf '%s\n' "$dir"
}

# start_agent <backend> <folder> <title> <permission-mode> <prompt> [claude args...]
start_agent() {
    local backend="$1" dir="$2" title="$3" mode="$4" prompt="$5"
    shift 5
    printf '%s\n' "$prompt" >"$dir/.prompt-$(date +%Y%m%d-%H%M%S).md" 2>/dev/null || true
    in_terminal pinnacle-agent "$title" \
        "$REPO/scripts/agent/run.sh" "$backend" "$(agent_profile)" "$dir" \
        --permission-mode "$mode" "$@" -- "$prompt"
}

# Asks for the prompt. The fuzzel prompt shows the backend that will run it.
ask_prompt() {
    local label="$1" backend="$2"
    ask_line --prompt "$label [$backend] › "
}

# Reads "--other" (the Shift+entry variant) from the arguments; prints the backend to use.
pick_backend() {
    if [[ "${1:-}" == --other ]]; then other_backend; else agent_backend; fi
}
