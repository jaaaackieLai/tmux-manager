#!/usr/bin/env bash
# lib/constants.sh - Constants, colors, and global state declarations

[[ -n "${_CONSTANTS_LOADED:-}" ]] && return
readonly _CONSTANTS_LOADED=1

readonly VERSION="1.2.1"
readonly GITHUB_REPO="jaaaackieLai/tmux-manager"
readonly GITHUB_RAW_BASE="https://raw.githubusercontent.com/${GITHUB_REPO}/main"
readonly PREVIEW_LINES=15
readonly PREVIEW_MAX_COLS=80
readonly CAPTURE_LINES=150
KEY_POLL_INTERVAL="0.2"
AI_TEMP_DIR="/tmp/tmux-manager-ai-$$"
readonly AI_MODEL="claude-haiku-4-5-20251001"

# ─── Colors and styles ───────────────────────────────────────────────
readonly RESET=$'\033[0m'
readonly BOLD=$'\033[1m'
readonly DIM=$'\033[2m'
readonly REVERSE=$'\033[7m'
readonly RED=$'\033[31m'
readonly GREEN=$'\033[32m'
readonly YELLOW=$'\033[33m'
readonly BLUE=$'\033[34m'
readonly CYAN=$'\033[36m'
readonly WHITE=$'\033[37m'
readonly BG_BLUE=$'\033[44m'
readonly GRAY=$'\033[90m'

# ─── Readline-safe styles (for read -p prompts) ─────────────────────
readonly RL_BOLD=$'\001\033[1m\002'
readonly RL_RESET=$'\001\033[0m\002'
readonly RL_DIM=$'\001\033[2m\002'

# ─── State ────────────────────────────────────────────────────────────
declare -a SESSIONS=()
declare -a AI_SUMMARIES=()
declare -a AI_NAMES=()
SELECTED=0
DETAIL_SELECTED=0
DIRTY=1
RUNNING=true
VIEW_MODE="list"    # "list" or "detail"
TERM_ROWS=0
TERM_COLS=0
LIST_END=0
readonly DETAIL_ACTIONS=("attach" "rename" "kill" "back")
readonly CAPTURE_PANES_FALLBACK="(unable to capture panes)"
declare -a WIN_COUNTS=()
declare -a PANE_COUNTS=()
