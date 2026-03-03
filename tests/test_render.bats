#!/usr/bin/env bats
# tests/test_render.bats - Tests for lib/render.sh

load 'test_helper'

setup() {
    # Source libs - note: declare -a in constants.sh becomes local inside
    # setup(), so we must re-assign globals AFTER sourcing all libs.
    source "${LIB_DIR}/constants.sh"
    source "${LIB_DIR}/utils.sh"
    source "${LIB_DIR}/sessions.sh"
    source "${LIB_DIR}/ai.sh"
    source "${LIB_DIR}/render.sh"

    # Stub non-buffered cursor/clear to suppress ANSI output
    cursor_to() { :; }
    clear_line() { :; }

    # Temp file for spying on capture_pane arguments
    SPY_FILE="$(mktemp)"
}

teardown() {
    rm -f "$SPY_FILE"
}

spy_capture_pane() {
    capture_pane() {
        echo "$2" > "$SPY_FILE"
        echo "line1"
    }
}

# ─── truncate_text tests ─────────────────────────────────────────────

@test "truncate_text returns text unchanged when shorter than max_len" {
    run truncate_text "hello" 10
    [ "$output" = "hello" ]
}

@test "truncate_text truncates and adds ellipsis when text exceeds max_len" {
    run truncate_text "hello world this is long" 10
    [ "$output" = "hello wor…" ]
}

@test "truncate_text returns text unchanged when exactly max_len" {
    run truncate_text "12345" 5
    [ "$output" = "12345" ]
}

@test "truncate_text handles empty string" {
    run truncate_text "" 10
    [ "$output" = "" ]
}

# ─── draw_session_list max_items tests ───────────────────────────────

@test "draw_session_list uses improved max_items formula for more visible sessions" {
    SESSIONS=("s1" "s2" "s3" "s4" "s5" "s6" "s7" "s8" "s9" "s10")
    AI_SUMMARIES=()
    SELECTED=0
    TERM_COLS=80
    TERM_ROWS=25
    ai_enabled() { return 1; }

    draw_session_list >/dev/null 2>&1
    # fixed_overhead=8, min_preview=3 -> max_items = 25 - 8 - 3 = 14, capped to 10
    # LIST_END = 3 + 10 = 13
    [ "$LIST_END" -eq 13 ]
}

@test "draw_session_list max_items minimum is 3 on very short terminal" {
    SESSIONS=("s1" "s2" "s3" "s4" "s5" "s6" "s7" "s8" "s9" "s10")
    AI_SUMMARIES=()
    SELECTED=0
    TERM_COLS=80
    TERM_ROWS=10
    ai_enabled() { return 1; }

    draw_session_list >/dev/null 2>&1
    # fixed_overhead=8, min_preview=3 -> max_items = 10 - 8 - 3 = -1, clamped to 3
    [ "$LIST_END" -eq 6 ]
}

# ─── draw_footer scroll indicator tests ──────────────────────────────

@test "draw_footer shows position indicator" {
    SESSIONS=("s1" "s2" "s3")
    SELECTED=1
    TERM_ROWS=30
    TERM_COLS=80
    _RENDER_BUF=""

    draw_footer
    output=$(buf_flush 2>&1)
    [[ "$output" == *"2/3"* ]]
}

# ─── render_detail kill color test ───────────────────────────────────

@test "render_detail applies red color to kill action" {
    SESSIONS=("test-session")
    AI_SUMMARIES=("")
    SELECTED=0
    DETAIL_SELECTED=2  # kill is index 2
    VIEW_MODE="detail"
    TERM_ROWS=25
    TERM_COLS=80
    _RENDER_BUF=""
    get_session_info() { echo "test: 1 window"; }
    ai_enabled() { return 1; }

    render_detail
    output=$(buf_flush 2>&1)
    # Output should contain RED escape for kill
    [[ "$output" == *$'\033[31m'* ]]
}

# ─── draw_detail_footer shortcut keys test ───────────────────────────

@test "draw_detail_footer shows attach rename kill shortcuts" {
    TERM_ROWS=25
    TERM_COLS=80
    _RENDER_BUF=""

    draw_detail_footer
    output=$(buf_flush 2>&1)
    [[ "$output" == *"[a]"* ]]
    [[ "$output" == *"[r]"* ]]
    [[ "$output" == *"[k]"* ]]
}

# ─── AI status display tests ────────────────────────────────────────

@test "draw_session_list shows AI failed when error file exists" {
    local tmpdir
    tmpdir=$(mktemp -d)
    SESSIONS=("test-session")
    AI_SUMMARIES=("")
    SELECTED=0
    TERM_COLS=80
    TERM_ROWS=25
    AI_TEMP_DIR="$tmpdir"
    _RENDER_BUF=""
    touch "${tmpdir}/test-session.error"
    ai_enabled() { return 0; }
    ai_has_error() { [[ -f "${AI_TEMP_DIR}/${1}.error" ]]; }

    draw_session_list
    output=$(buf_flush 2>&1)
    [[ "$output" == *"AI failed"* ]]

    rm -rf "$tmpdir"
}

# ─── draw_preview tests ─────────────────────────────────────────────

@test "draw_preview passes dynamic line count to capture_pane based on terminal height" {
    SESSIONS=("test-session")
    SELECTED=0
    TERM_COLS=80
    TERM_ROWS=22
    # preview_start=10, available = 22 - 10 - 5 = 7, which is < PREVIEW_LINES(15)
    local expected_lines=7
    spy_capture_pane

    draw_preview 10

    [ "$(cat "$SPY_FILE")" -eq "$expected_lines" ]
}

@test "draw_preview caps line count at PREVIEW_LINES when terminal is very tall" {
    SESSIONS=("test-session")
    SELECTED=0
    TERM_COLS=80
    TERM_ROWS=200
    spy_capture_pane

    draw_preview 10

    [ "$(cat "$SPY_FILE")" -eq "$PREVIEW_LINES" ]
}

@test "draw_preview uses minimum 3 lines when terminal is very short" {
    SESSIONS=("test-session")
    SELECTED=0
    TERM_COLS=80
    TERM_ROWS=12
    spy_capture_pane

    # preview_start=5, available = 12 - 5 - 5 = 2, clamped to 3
    draw_preview 5

    [ "$(cat "$SPY_FILE")" -eq 3 ]
}

@test "draw_preview clears residual lines when terminal shrinks below preview threshold" {
    SESSIONS=("test-session")
    SELECTED=0
    TERM_COLS=80
    TERM_ROWS=13
    _RENDER_BUF=""
    # preview_start=8, available = 13 - 8 - 5 = 0 -> no preview content
    # But row 9 (TERM_ROWS-4) should still be cleared to remove stale content
    capture_pane() { echo "stale"; }

    draw_preview 8
    output=$(buf_flush 2>&1)

    # The preview header row should be rendered (cursor move to row 8)
    [[ "$output" == *$'\033[8;1H'* ]]
    # Row 9 = TERM_ROWS-4 should be cleared (cursor move to row 9)
    [[ "$output" == *$'\033[9;1H'* ]]
}

# ─── buf_* output buffering tests ───────────────────────────────────

# ─── tmux tips tests ───────────────────────────────────────────────

@test "draw_footer shows tmux tips line" {
    SESSIONS=("s1" "s2")
    SELECTED=0
    TERM_ROWS=25
    TERM_COLS=80
    _RENDER_BUF=""
    _LIST_OFFSET=0
    _LIST_MAX_ITEMS=2

    draw_footer
    output=$(buf_flush 2>&1)
    [[ "$output" == *"C-b d"* ]]
}

@test "draw_detail_footer shows tmux tips line" {
    TERM_ROWS=25
    TERM_COLS=80
    _RENDER_BUF=""

    draw_detail_footer
    output=$(buf_flush 2>&1)
    [[ "$output" == *"C-b d"* ]]
}

@test "draw_footer hides tmux tips on very small terminal" {
    SESSIONS=("s1")
    SELECTED=0
    TERM_ROWS=8
    TERM_COLS=80
    _RENDER_BUF=""
    _LIST_OFFSET=0
    _LIST_MAX_ITEMS=1

    draw_footer
    output=$(buf_flush 2>&1)
    [[ "$output" != *"C-b d"* ]]
}

# ─── AI summary printf safety tests ──────────────────────────────────

@test "draw_session_list handles percent in AI summary without crashing" {
    SESSIONS=("test-session")
    AI_SUMMARIES=("完成 80% 測試")
    SELECTED=0
    TERM_COLS=80
    TERM_ROWS=25
    _RENDER_BUF=""
    ai_enabled() { return 0; }

    draw_session_list
    output=$(buf_flush 2>&1)
    [[ "$output" == *"80%"* ]]
}

# ─── buf_* output buffering tests ───────────────────────────────────

@test "buf_print appends text to _RENDER_BUF" {
    _RENDER_BUF=""
    buf_print "hello"
    buf_print " world"
    [ "$_RENDER_BUF" = "hello world" ]
}

@test "buf_printf formats text into _RENDER_BUF" {
    _RENDER_BUF=""
    buf_printf "num=%d" 42
    [ "$_RENDER_BUF" = "num=42" ]
}

@test "buf_flush outputs buffer and clears it" {
    run bash -c "
        source '${LIB_DIR}/utils.sh'
        _RENDER_BUF='test output'
        buf_flush
        printf '\n'
        echo \"AFTER:\${_RENDER_BUF}:END\"
    "
    [ "$status" -eq 0 ]
    [ "${lines[0]}" = "test output" ]
    [ "${lines[1]}" = "AFTER::END" ]
}

@test "buf_cursor_to appends ANSI cursor sequence to buffer" {
    _RENDER_BUF=""
    buf_cursor_to 5 10
    [ "$_RENDER_BUF" = $'\033[5;10H' ]
}

@test "buf_clear_line appends ANSI clear sequence to buffer" {
    _RENDER_BUF=""
    buf_clear_line
    [ "$_RENDER_BUF" = $'\033[2K' ]
}
