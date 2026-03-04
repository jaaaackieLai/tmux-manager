#!/usr/bin/env bash
# lib/sessions.sh - tmux session management

refresh_sessions() {
    SESSIONS=()
    AI_SUMMARIES=()
    AI_NAMES=()
    WIN_COUNTS=()
    PANE_COUNTS=()

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

    # Cache window/pane counts for all sessions
    local i
    for i in "${!SESSIONS[@]}"; do
        local wc pc
        {
            IFS= read -r wc
            IFS= read -r pc
        } < <(get_pane_window_counts "${SESSIONS[$i]}")
        WIN_COUNTS+=("${wc:-?}")
        PANE_COUNTS+=("${pc:-?}")
    done
}

get_session_info() {
    local session="$1"
    local result
    result=$(tmux ls -F '#{session_name}: #{session_windows} windows (created #{session_created_string})#{?session_attached, (attached),}' 2>/dev/null \
        | awk -F': ' -v s="$session" '$1 == s' | head -1)
    echo "${result:-${session}: unknown}"
}

get_pane_window_counts() {
    local session="$1"
    local pane_rows
    pane_rows=$(tmux list-panes -s -t "$session" -F '#{window_index}' 2>/dev/null) || pane_rows=""
    if [[ -z "$pane_rows" ]]; then
        echo "?"
        echo "?"
        return
    fi

    awk 'NF { panes++; if (!seen[$0]++) windows++ } END { print windows+0; print panes+0 }' <<< "$pane_rows"
}

capture_all_panes() {
    local session="$1"
    local total_budget="${2:-$CAPTURE_LINES}"
    if ! [[ "$total_budget" =~ ^[0-9]+$ ]] || (( total_budget < 1 )); then
        total_budget=1
    fi

    local pane_fmt pane_rows
    pane_fmt=$'#{pane_id}\t#{window_index}\t#{pane_index}\t#{window_name}'
    pane_rows=$(tmux list-panes -s -t "$session" -F "$pane_fmt" 2>/dev/null) || {
        echo "$CAPTURE_PANES_FALLBACK"
        return
    }
    [[ -z "$pane_rows" ]] && {
        echo "$CAPTURE_PANES_FALLBACK"
        return
    }

    local -a pane_entries=()
    mapfile -t pane_entries <<< "$pane_rows"
    local total_panes=${#pane_entries[@]}
    (( total_panes > 0 )) || {
        echo "$CAPTURE_PANES_FALLBACK"
        return
    }

    local header_per_pane=1
    local usable=$(( total_budget - total_panes * header_per_pane ))
    (( usable < total_panes )) && usable=$total_panes
    local per_pane=$(( usable / total_panes ))
    (( per_pane < 1 )) && per_pane=1

    local emitted=0 out=""
    local pane_line pane_id win_idx pane_idx win_name
    for pane_line in "${pane_entries[@]}"; do
        IFS=$'\t' read -r pane_id win_idx pane_idx win_name <<< "$pane_line"
        [[ -n "$pane_id" && -n "$win_idx" && -n "$pane_idx" ]] || continue

        emitted=1
        out+="=== Window ${win_idx} (${win_name}) / Pane ${pane_idx} ==="$'\n'

        local pane_text
        pane_text=$(tmux capture-pane -t "$pane_id" -p -S "-${per_pane}" 2>/dev/null) \
            || pane_text="(unable to capture pane ${win_idx}.${pane_idx})"
        out+="${pane_text}"$'\n'
    done

    (( emitted )) || {
        echo "$CAPTURE_PANES_FALLBACK"
        return
    }

    awk -v n="$total_budget" 'NR <= n { print }' <<< "$out"
}

capture_pane() {
    local session="$1"
    local lines="${2:-$CAPTURE_LINES}"
    tmux capture-pane -t "$session" -p -S "-${lines}" 2>/dev/null || echo "(unable to capture pane)"
}
