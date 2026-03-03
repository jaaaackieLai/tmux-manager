#!/usr/bin/env bats
# tests/test_input.bats - Tests for lib/input.sh

load 'test_helper'

# ─── read_key tests ─────────────────────────────────────────────────

@test "read_key returns UP for up arrow escape sequence" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        printf '\033[A' | read_key
    "
    [ "$status" -eq 0 ]
    [ "$output" = "UP" ]
}

@test "read_key returns DOWN for down arrow escape sequence" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        printf '\033[B' | read_key
    "
    [ "$status" -eq 0 ]
    [ "$output" = "DOWN" ]
}

@test "read_key returns ENTER for newline (bash strips it to empty string)" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        printf '\n' | read_key
    "
    [ "$status" -eq 0 ]
    [ "$output" = "ENTER" ]
}

@test "read_key returns ESC for lone escape" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        printf '\033' | read_key
    "
    [ "$status" -eq 0 ]
    [ "$output" = "ESC" ]
}

@test "read_key returns TIMEOUT on no input" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        read_key </dev/null
    "
    [ "$status" -eq 0 ]
    [ "$output" = "TIMEOUT" ]
}

# ─── SELECTED boundary tests via handle_input ────────────────────────

@test "UP key wraps to last session when SELECTED is 0" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        read_key() { echo 'UP'; }
        render() { :; }
        handle_input
        echo \"\$SELECTED\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "2" ]
}

@test "DOWN key wraps to first session when at last index" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=2
        read_key() { echo 'DOWN'; }
        render() { :; }
        handle_input
        echo \"\$SELECTED\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "0" ]
}

@test "UP key decrements SELECTED from middle" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=2
        read_key() { echo 'UP'; }
        render() { :; }
        handle_input
        echo \"\$SELECTED\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "1" ]
}

@test "DOWN key increments SELECTED from middle" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=1
        read_key() { echo 'DOWN'; }
        render() { :; }
        handle_input
        echo \"\$SELECTED\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "2" ]
}

# ─── handle_input: ENTER switches to detail mode ─────────────────────

@test "ENTER key in list mode sets VIEW_MODE to detail" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        VIEW_MODE='list'
        read_key() { echo 'ENTER'; }
        render() { :; }
        handle_input
        echo \"\$VIEW_MODE\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "detail" ]
}

@test "r key in list mode is ignored (no action)" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        VIEW_MODE='list'
        read_key() { echo 'r'; }
        render() { :; }
        action_rename() { echo 'RENAMED'; }
        handle_input
        echo \"\$VIEW_MODE\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "list" ]
}

@test "k key in list mode is ignored (no action)" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        VIEW_MODE='list'
        read_key() { echo 'k'; }
        render() { :; }
        action_kill() { echo 'KILLED'; }
        handle_input
        echo \"\$VIEW_MODE\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "list" ]
}

# ─── handle_detail_input tests ───────────────────────────────────────

@test "ESC in detail mode sets VIEW_MODE back to list" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        VIEW_MODE='detail'
        read_key() { echo 'ESC'; }
        render() { :; }
        handle_detail_input
        echo \"\$VIEW_MODE\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "list" ]
}

@test "q in detail mode sets VIEW_MODE back to list" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        VIEW_MODE='detail'
        read_key() { echo 'q'; }
        render() { :; }
        handle_detail_input
        echo \"\$VIEW_MODE\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "list" ]
}

@test "UP in detail mode decrements DETAIL_SELECTED" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        DETAIL_SELECTED=2
        VIEW_MODE='detail'
        read_key() { echo 'UP'; }
        render() { :; }
        handle_detail_input
        echo \"\$DETAIL_SELECTED\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "1" ]
}

@test "DOWN in detail mode increments DETAIL_SELECTED" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        DETAIL_SELECTED=1
        VIEW_MODE='detail'
        read_key() { echo 'DOWN'; }
        render() { :; }
        handle_detail_input
        echo \"\$DETAIL_SELECTED\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "2" ]
}

@test "UP in detail mode wraps to last action when at 0" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        DETAIL_SELECTED=0
        VIEW_MODE='detail'
        read_key() { echo 'UP'; }
        render() { :; }
        handle_detail_input
        echo \"\$DETAIL_SELECTED\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "3" ]
}

@test "DOWN in detail mode wraps to first action when at last" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        DETAIL_SELECTED=3
        VIEW_MODE='detail'
        read_key() { echo 'DOWN'; }
        render() { :; }
        handle_detail_input
        echo \"\$DETAIL_SELECTED\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "0" ]
}

@test "ENTER in detail mode with DETAIL_SELECTED=0 calls action_attach" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        DETAIL_SELECTED=0
        VIEW_MODE='detail'
        read_key() { echo 'ENTER'; }
        render() { :; }
        action_attach() { echo 'ATTACHED'; }
        handle_detail_input
    "
    [ "$status" -eq 0 ]
    [[ "$output" == *"ATTACHED"* ]]
}

@test "ENTER in detail mode with DETAIL_SELECTED=1 calls action_rename" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        DETAIL_SELECTED=1
        VIEW_MODE='detail'
        read_key() { echo 'ENTER'; }
        render() { :; }
        action_rename() { echo 'RENAMED'; }
        handle_detail_input
    "
    [ "$status" -eq 0 ]
    [[ "$output" == *"RENAMED"* ]]
}

@test "ENTER in detail mode with DETAIL_SELECTED=2 calls action_kill" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        DETAIL_SELECTED=2
        VIEW_MODE='detail'
        read_key() { echo 'ENTER'; }
        render() { :; }
        action_kill() { echo 'KILLED'; }
        handle_detail_input
    "
    [ "$status" -eq 0 ]
    [[ "$output" == *"KILLED"* ]]
}

@test "ENTER in detail mode with DETAIL_SELECTED=3 sets VIEW_MODE to list" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        DETAIL_SELECTED=3
        VIEW_MODE='detail'
        read_key() { echo 'ENTER'; }
        render() { :; }
        handle_detail_input
        echo \"\$VIEW_MODE\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "list" ]
}

# ─── Detail view shortcut key tests (a/r/k) ──────────────────────────

@test "a key in detail mode calls action_attach" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        DETAIL_SELECTED=0
        VIEW_MODE='detail'
        read_key() { echo 'a'; }
        render() { :; }
        action_attach() { echo 'ATTACHED'; }
        handle_detail_input
    "
    [ "$status" -eq 0 ]
    [[ "$output" == *"ATTACHED"* ]]
}

@test "r key in detail mode calls action_rename" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        DETAIL_SELECTED=0
        VIEW_MODE='detail'
        read_key() { echo 'r'; }
        render() { :; }
        action_rename() { echo 'RENAMED'; }
        handle_detail_input
    "
    [ "$status" -eq 0 ]
    [[ "$output" == *"RENAMED"* ]]
}

@test "k key in detail mode calls action_kill" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        DETAIL_SELECTED=0
        VIEW_MODE='detail'
        read_key() { echo 'k'; }
        render() { :; }
        action_kill() { echo 'KILLED'; }
        handle_detail_input
    "
    [ "$status" -eq 0 ]
    [[ "$output" == *"KILLED"* ]]
}

# ─── DIRTY flag tests ───────────────────────────────────────────────

@test "TIMEOUT in list mode always sets DIRTY=1" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta)
        AI_SUMMARIES=('sum1' 'sum2')
        AI_TEMP_DIR=\$(mktemp -d)
        echo 'sum1' > \"\${AI_TEMP_DIR}/alpha.summary\"
        echo 'sum2' > \"\${AI_TEMP_DIR}/beta.summary\"
        DIRTY=0
        read_key() { echo 'TIMEOUT'; }
        render() { :; }
        handle_input
        echo \"\$DIRTY\"
        rm -rf \"\$AI_TEMP_DIR\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "1" ]
}

@test "TIMEOUT in detail mode always sets DIRTY=1" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha)
        SELECTED=0
        VIEW_MODE='detail'
        AI_TEMP_DIR='/nonexistent/path'
        DIRTY=0
        read_key() { echo 'TIMEOUT'; }
        render() { :; }
        handle_detail_input
        echo \"\$DIRTY\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "1" ]
}

@test "DIRTY set to 1 on UP key press in list mode" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=1
        DIRTY=0
        read_key() { echo 'UP'; }
        render() { :; }
        handle_input
        echo \"\$DIRTY\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "1" ]
}

@test "DIRTY set to 1 on DOWN key press in detail mode" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        DETAIL_SELECTED=0
        VIEW_MODE='detail'
        DIRTY=0
        read_key() { echo 'DOWN'; }
        render() { :; }
        handle_detail_input
        echo \"\$DIRTY\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "1" ]
}

@test "read_key returns TAB for tab character" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        printf '\t' | read_key
    "
    [ "$status" -eq 0 ]
    [ "$output" = "TAB" ]
}

@test "TAB in list mode increments SELECTED" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        read_key() { echo 'TAB'; }
        render() { :; }
        handle_input
        echo \"\$SELECTED\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "1" ]
}

@test "TAB in list mode wraps to first session when at last" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=2
        read_key() { echo 'TAB'; }
        render() { :; }
        handle_input
        echo \"\$SELECTED\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "0" ]
}

@test "TAB in detail mode increments DETAIL_SELECTED" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        DETAIL_SELECTED=1
        VIEW_MODE='detail'
        read_key() { echo 'TAB'; }
        render() { :; }
        handle_detail_input
        echo \"\$DETAIL_SELECTED\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "2" ]
}

@test "TAB in detail mode wraps to first action when at last" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha beta gamma)
        SELECTED=0
        DETAIL_SELECTED=3
        VIEW_MODE='detail'
        read_key() { echo 'TAB'; }
        render() { :; }
        handle_detail_input
        echo \"\$DETAIL_SELECTED\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "0" ]
}

@test "TIMEOUT in list mode sets DIRTY=1 even with no AI dir" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'
        source '${LIB_DIR}/input.sh'
        SESSIONS=(alpha)
        AI_TEMP_DIR='/nonexistent/path'
        DIRTY=0
        read_key() { echo 'TIMEOUT'; }
        render() { :; }
        handle_input
        echo \"\$DIRTY\"
    "
    [ "$status" -eq 0 ]
    [ "$output" = "1" ]
}
