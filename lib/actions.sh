#!/usr/bin/env bash
# lib/actions.sh - User actions (attach, rename, kill, new)

action_attach() {
    if [[ ${#SESSIONS[@]} -eq 0 ]]; then return; fi
    local session="${SESSIONS[$SELECTED]}"
    cursor_show
    clear_screen
    # Restore terminal before attaching
    stty "$SAVED_TTY" 2>/dev/null || true
    tmux attach -t "$session"
    # After detach, re-setup terminal
    SAVED_TTY=$(stty -g 2>/dev/null || true)
    stty -echo -icanon min 1 time 0 2>/dev/null || true
    cursor_hide
    VIEW_MODE="list"
    refresh_sessions
    DIRTY=1
}

action_rename() {
    if [[ ${#SESSIONS[@]} -eq 0 ]]; then return; fi
    local session="${SESSIONS[$SELECTED]}"
    local suggested="${AI_NAMES[$SELECTED]:-}"

    # Show rename prompt
    cursor_show
    local prompt_row=$(( TERM_ROWS ))
    cursor_to "$prompt_row" 1
    clear_line

    local new_name=""

    if [[ -n "$suggested" ]]; then
        printf " ${BOLD}Rename '%s'${RESET} ${DIM}[Enter] AI: %s  [e] custom  [ESC] cancel${RESET}" "$session" "$suggested"
        local choice=""
        IFS= read -r -s -n 1 choice 2>/dev/null || true

        case "$choice" in
            ""|$'\n'|$'\r')
                new_name="$suggested"
                ;;
            e|E)
                stty "$SAVED_TTY" 2>/dev/null || true
                stty echo icanon 2>/dev/null || true
                cursor_to "$prompt_row" 1
                clear_line
                IFS= read -r -e -p " ${RL_BOLD}Custom name${RL_RESET} ${RL_DIM}(AI: ${suggested}, empty=cancel): ${RL_RESET}" new_name || true
                ;;
            $'\x1b'|q|Q)
                new_name=""
                ;;
            *)
                new_name=""
                ;;
        esac
    else
        # Restore terminal for line input
        stty "$SAVED_TTY" 2>/dev/null || true
        stty echo icanon 2>/dev/null || true
        IFS= read -r -e -p " ${RL_BOLD}Rename '${session}' to${RL_RESET} ${RL_DIM}(empty=cancel): ${RL_RESET}" new_name || true
    fi

    # Re-setup raw terminal
    stty -echo -icanon min 1 time 0 2>/dev/null || true
    cursor_hide

    # Clear the prompt line
    cursor_to "$prompt_row" 1
    clear_line

    if [[ -n "$new_name" ]]; then
        tmux rename-session -t "$session" "$new_name" 2>/dev/null || true
        refresh_sessions
    fi
    VIEW_MODE="list"
    DIRTY=1
}

action_kill() {
    if [[ ${#SESSIONS[@]} -eq 0 ]]; then return; fi
    local session="${SESSIONS[$SELECTED]}"

    # Show confirm prompt (stay in raw mode, read single key without Enter)
    local prompt_row=$(( TERM_ROWS ))
    cursor_to "$prompt_row" 1
    clear_line
    printf " ${RED}${BOLD}Kill session '${session}'? [y/N]${RESET} "

    local confirm=""
    IFS= read -r -s -n 1 confirm 2>/dev/null || true

    # Clear the prompt line
    cursor_to "$prompt_row" 1
    clear_line

    if [[ "$confirm" == "y" || "$confirm" == "Y" ]]; then
        tmux kill-session -t "$session" 2>/dev/null || true
        refresh_sessions
    fi
    VIEW_MODE="list"
    DIRTY=1
}

create_session_with_context() {
    local name="$1"
    local workdir="${2:-}"
    local init_cmd="${3:-}"

    if [[ -n "$workdir" ]]; then
        tmux new-session -d -s "$name" -c "$workdir" 2>/dev/null || tmux new-session -d -s "$name" 2>/dev/null || return 1
    else
        tmux new-session -d -s "$name" 2>/dev/null || return 1
    fi

    if [[ -n "$init_cmd" ]]; then
        tmux send-keys -t "$name" "$init_cmd" C-m 2>/dev/null || true
    fi
}

action_new() {
    cursor_show
    local prompt_row=$(( TERM_ROWS ))
    cursor_to "$prompt_row" 1
    clear_line

    stty "$SAVED_TTY" 2>/dev/null || true
    stty echo icanon 2>/dev/null || true

    local name=""
    IFS= read -r -e -p " ${RL_BOLD}New session name${RL_RESET} (empty=cancel): " name || true

    local workdir="${TMUX_MANAGER_NEW_DEFAULT_DIR:-}"
    local init_cmd="${TMUX_MANAGER_NEW_DEFAULT_CMD:-}"
    local ask_dir="${TMUX_MANAGER_NEW_ASK_DIR:-0}"
    local ask_cmd="${TMUX_MANAGER_NEW_ASK_CMD:-0}"

    if [[ -n "$name" ]]; then
        if [[ "$ask_dir" == "1" ]]; then
            local dir_hint="empty=skip"
            [[ -n "$workdir" ]] && dir_hint="empty=${workdir}"
            local input_dir=""
            IFS= read -r -e -p " ${RL_BOLD}Workdir${RL_RESET} ${RL_DIM}(${dir_hint}): ${RL_RESET}" input_dir || true
            if [[ -n "$input_dir" ]]; then
                workdir="$input_dir"
            fi
        fi

        if [[ "$ask_cmd" == "1" ]]; then
            local cmd_hint="empty=none"
            [[ -n "$init_cmd" ]] && cmd_hint="empty=${init_cmd}, '-'=none"
            local input_cmd=""
            IFS= read -r -e -p " ${RL_BOLD}Init command${RL_RESET} ${RL_DIM}(${cmd_hint}): ${RL_RESET}" input_cmd || true
            if [[ "$input_cmd" == "-" ]]; then
                init_cmd=""
            elif [[ -n "$input_cmd" ]]; then
                init_cmd="$input_cmd"
            fi
        fi
    fi

    stty -echo -icanon min 1 time 0 2>/dev/null || true
    cursor_hide

    # Clear the prompt line
    cursor_to "$prompt_row" 1
    clear_line

    if [[ -n "$name" ]]; then
        if create_session_with_context "$name" "$workdir" "$init_cmd"; then
            refresh_sessions
            # Find the new session and attach directly
            for i in "${!SESSIONS[@]}"; do
                if [[ "${SESSIONS[$i]}" == "$name" ]]; then
                    SELECTED=$i
                    break
                fi
            done
            action_attach
            return
        fi
    fi
    DIRTY=1
}
