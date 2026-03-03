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
