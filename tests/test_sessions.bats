#!/usr/bin/env bats
# tests/test_sessions.bats - Tests for lib/sessions.sh

load 'test_helper'

setup() {
    source "${LIB_DIR}/constants.sh"
    source "${LIB_DIR}/utils.sh"
    source "${LIB_DIR}/sessions.sh"
}

# Helper: mock tmux to return 3 fake sessions
mock_tmux_three_sessions() {
    tmux() {
        if [[ "${1:-}" == "ls" ]]; then
            printf 'alpha: 1 windows\nbeta: 2 windows\ngamma: 1 windows\n'
        fi
    }
    export -f tmux
}

# Helper: mock tmux to fail (no sessions)
mock_tmux_no_sessions() {
    tmux() { return 1; }
    export -f tmux
}

@test "refresh_sessions populates SESSIONS array" {
    mock_tmux_three_sessions
    refresh_sessions
    [ "${#SESSIONS[@]}" -eq 3 ]
    [ "${SESSIONS[0]}" = "alpha" ]
    [ "${SESSIONS[1]}" = "beta" ]
    [ "${SESSIONS[2]}" = "gamma" ]
}

@test "refresh_sessions clears previous AI data" {
    mock_tmux_three_sessions
    AI_SUMMARIES=("old1" "old2")
    AI_NAMES=("name1" "name2")
    refresh_sessions
    [ "${AI_SUMMARIES[0]}" = "" ]
    [ "${AI_NAMES[0]}" = "" ]
}

@test "refresh_sessions clamps SELECTED when above range" {
    mock_tmux_three_sessions
    SELECTED=99
    refresh_sessions
    [ "$SELECTED" -eq 2 ]
}

@test "refresh_sessions clamps SELECTED to 0 when negative" {
    mock_tmux_three_sessions
    SELECTED=-5
    refresh_sessions
    [ "$SELECTED" -eq 0 ]
}

@test "refresh_sessions keeps SELECTED when within range" {
    mock_tmux_three_sessions
    SELECTED=1
    refresh_sessions
    [ "$SELECTED" -eq 1 ]
}

@test "refresh_sessions handles no sessions gracefully" {
    mock_tmux_no_sessions
    SELECTED=0
    refresh_sessions
    [ "${#SESSIONS[@]}" -eq 0 ]
    [ "$SELECTED" -eq 0 ]
}

@test "get_pane_window_counts returns total windows and panes" {
    local call_log="${BATS_TMPDIR}/pane-count-calls-$$"
    : > "$call_log"
    tmux() {
        case "${1:-}" in
            list-panes)
                echo "list-panes" >> "$call_log"
                printf '1\n1\n2\n'
                ;;
            list-windows)
                echo "list-windows" >> "$call_log"
                return 1
                ;;
        esac
    }
    export -f tmux

    run get_pane_window_counts "alpha"
    [ "$status" -eq 0 ]
    [ "${lines[0]}" = "2" ]
    [ "${lines[1]}" = "3" ]
    [ "$(grep -c '^list-panes$' "$call_log")" -eq 1 ]
    [ "$(grep -c '^list-windows$' "$call_log" || true)" -eq 0 ]
}

@test "get_pane_window_counts returns unknown fallback when tmux fails" {
    tmux() { return 1; }
    export -f tmux

    run get_pane_window_counts "alpha"
    [ "$status" -eq 0 ]
    [ "${lines[0]}" = "?" ]
    [ "${lines[1]}" = "?" ]
}

@test "capture_all_panes captures all panes with section headers" {
    tmux() {
        case "${1:-}" in
            list-panes)
                printf '%%1\t1\t1\tmain\n'
                printf '%%2\t1\t2\tmain\n'
                printf '%%3\t2\t1\tssh:user@host:2222\n'
                ;;
            capture-pane)
                case "${3:-}" in
                    %1) echo "compile" ;;
                    %2) echo "test" ;;
                    %3) echo "tail -f app.log" ;;
                esac
                ;;
        esac
    }
    export -f tmux

    run capture_all_panes "alpha" 30
    [ "$status" -eq 0 ]
    [[ "$output" == *"=== Window 1 (main) / Pane 1 ==="* ]]
    [[ "$output" == *"=== Window 1 (main) / Pane 2 ==="* ]]
    [[ "$output" == *"=== Window 2 (ssh:user@host:2222) / Pane 1 ==="* ]]
    [[ "$output" == *"compile"* ]]
    [[ "$output" == *"test"* ]]
    [[ "$output" == *"tail -f app.log"* ]]
}

@test "capture_all_panes keeps output within total budget" {
    tmux() {
        case "${1:-}" in
            list-panes)
                printf '%%1\t1\t1\tmain\n'
                printf '%%2\t1\t2\tmain\n'
                printf '%%3\t1\t3\tmain\n'
                ;;
            capture-pane)
                printf 'line-a\nline-b\nline-c\nline-d\n'
                ;;
        esac
    }
    export -f tmux

    run capture_all_panes "alpha" 4
    [ "$status" -eq 0 ]
    line_count=$(printf '%s' "$output" | awk 'END { print NR }')
    [ "$line_count" -le 4 ]
}

@test "capture_all_panes returns fallback when pane listing emits nothing" {
    tmux() {
        case "${1:-}" in
            list-panes)
                return 1
                ;;
        esac
    }
    export -f tmux

    run capture_all_panes "alpha" 20
    [ "$status" -eq 0 ]
    [ "$output" = "(unable to capture panes)" ]
}

@test "capture_all_panes returns fallback when list-panes returns empty" {
    tmux() {
        if [[ "${1:-}" == "list-panes" ]]; then
            return 0
        fi
    }
    export -f tmux

    run capture_all_panes "alpha" 20
    [ "$status" -eq 0 ]
    [ "$output" = "(unable to capture panes)" ]
}
