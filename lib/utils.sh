#!/usr/bin/env bash
# lib/utils.sh - Terminal utilities and helpers

die() {
    echo "Error: $1" >&2
    exit 1
}

check_deps() {
    command -v tmux >/dev/null 2>&1 || die "tmux is not installed"
}

cursor_hide() { printf '\033[?25l'; }
cursor_show() { printf '\033[?25h'; }
cursor_to() { printf '\033[%d;%dH' "$1" "$2"; }
clear_screen() { printf '\033[2J\033[H'; }
clear_line() { printf '\033[2K'; }
clear_below() { printf '\033[J'; }

get_term_size() {
    TERM_ROWS=$(tput lines 2>/dev/null || echo 24)
    TERM_COLS=$(tput cols 2>/dev/null || echo 80)
}

# ─── Output buffering ────────────────────────────────────────────────
_RENDER_BUF=""
buf_print()      { _RENDER_BUF+="$*"; }
buf_printf()     { _RENDER_BUF+="$(printf "$@")"; }
buf_flush()      { printf '%s' "$_RENDER_BUF"; _RENDER_BUF=""; }
buf_cursor_to()  { _RENDER_BUF+=$'\033['"${1};${2}H"; }
buf_clear_line() { _RENDER_BUF+=$'\033[2K'; }
