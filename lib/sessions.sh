#!/usr/bin/env bash
# lib/sessions.sh - tmux session management

refresh_sessions() {
    SESSIONS=()
    AI_SUMMARIES=()
    AI_NAMES=()

    local line
    while IFS= read -r line; do
        # Extract session name (before the colon)
        local name="${line%%:*}"
        SESSIONS+=("$name")
        AI_SUMMARIES+=("")
        AI_NAMES+=("")
    done < <(tmux ls 2>/dev/null || true)

    if [[ ${#SESSIONS[@]} -eq 0 ]]; then
        return 0
    fi

    # Clamp selected index
    if (( SELECTED >= ${#SESSIONS[@]} )); then
        SELECTED=$(( ${#SESSIONS[@]} - 1 ))
    fi
    if (( SELECTED < 0 )); then
        SELECTED=0
    fi
}

get_session_info() {
    local session="$1"
    local result
    result=$(tmux ls -F '#{session_name}: #{session_windows} windows (created #{session_created_string})#{?session_attached, (attached),}' 2>/dev/null \
        | awk -F': ' -v s="$session" '$1 == s' | head -1)
    echo "${result:-${session}: unknown}"
}

capture_pane() {
    local session="$1"
    local lines="${2:-$CAPTURE_LINES}"
    tmux capture-pane -t "$session" -p -S "-${lines}" 2>/dev/null || echo "(unable to capture pane)"
}
