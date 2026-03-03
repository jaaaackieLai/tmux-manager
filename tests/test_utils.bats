#!/usr/bin/env bats
# tests/test_utils.bats - Tests for lib/utils.sh

load 'test_helper'

setup() {
    load_lib constants
    load_lib utils
}

@test "cursor_to outputs correct ANSI escape sequence" {
    run bash -c "source '${LIB_DIR}/constants.sh'; source '${LIB_DIR}/utils.sh'; cursor_to 3 5"
    [ "$status" -eq 0 ]
    [ "$output" = $'\033[3;5H' ]
}

@test "cursor_to row 1 col 1" {
    run bash -c "source '${LIB_DIR}/constants.sh'; source '${LIB_DIR}/utils.sh'; cursor_to 1 1"
    [ "$status" -eq 0 ]
    [ "$output" = $'\033[1;1H' ]
}

@test "die exits with code 1" {
    run bash -c "source '${LIB_DIR}/constants.sh'; source '${LIB_DIR}/utils.sh'; die 'test error'"
    [ "$status" -eq 1 ]
    [[ "$output" == *"Error: test error"* ]]
}

@test "cursor_hide outputs hide sequence" {
    run bash -c "source '${LIB_DIR}/constants.sh'; source '${LIB_DIR}/utils.sh'; cursor_hide"
    [ "$status" -eq 0 ]
    [ "$output" = $'\033[?25l' ]
}

@test "cursor_show outputs show sequence" {
    run bash -c "source '${LIB_DIR}/constants.sh'; source '${LIB_DIR}/utils.sh'; cursor_show"
    [ "$status" -eq 0 ]
    [ "$output" = $'\033[?25h' ]
}
