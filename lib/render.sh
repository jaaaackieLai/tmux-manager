#!/usr/bin/env bash
# lib/render.sh - TUI rendering functions (buffered output)

# Truncate text with ellipsis when exceeding max_len
truncate_text() {
    local text="$1" max_len="$2"
    if (( ${#text} > max_len )); then
        printf '%s' "${text:0:$(( max_len - 1 ))}"$'\xe2\x80\xa6'
    else
        printf '%s' "$text"
    fi
}

draw_header() {
    buf_cursor_to 1 1
    buf_clear_line
    buf_printf "${BOLD}${CYAN} tmux-manager${RESET}${DIM}  v${VERSION}${RESET}"
    buf_cursor_to 2 1
    buf_clear_line
    local separator=""
    local max_w=$(( TERM_COLS - 2 ))
    for (( c=0; c<max_w; c++ )); do separator+="─"; done
    buf_printf " ${DIM}${separator}${RESET}"
}

draw_session_list() {
    local start_row=3
    # Fixed overhead: header(2) + separator(1) + preview_header(1) + separator(1) + footer(3) = 8
    local fixed_overhead=8
    local min_preview=3
    local max_items=$(( TERM_ROWS - fixed_overhead - min_preview ))
    if (( max_items < 3 )); then max_items=3; fi
    if (( max_items > ${#SESSIONS[@]} )); then max_items=${#SESSIONS[@]}; fi

    # Calculate scroll offset
    local offset=0
    if (( SELECTED >= max_items )); then
        offset=$(( SELECTED - max_items + 1 ))
    fi

    # Dynamic column widths for truncation
    local name_max=$(( TERM_COLS / 3 ))
    if (( name_max < 15 )); then name_max=15; fi
    local ai_max=$(( TERM_COLS - name_max - 10 ))
    if (( ai_max < 10 )); then ai_max=10; fi

    local total=${#SESSIONS[@]}
    local i
    for (( i=0; i<max_items; i++ )); do
        local idx=$(( offset + i ))
        if (( idx >= total )); then break; fi

        local row=$(( start_row + i ))
        buf_cursor_to "$row" 1
        buf_clear_line

        local session="${SESSIONS[$idx]}"
        local display_name
        display_name=$(truncate_text "$session" "$name_max")

        local ai_text=""
        if [[ -n "${AI_SUMMARIES[$idx]:-}" ]]; then
            local truncated_ai
            truncated_ai=$(truncate_text "${AI_SUMMARIES[$idx]}" "$ai_max")
            ai_text="${CYAN} [${truncated_ai}]${RESET}"
        elif ai_enabled; then
            if ai_has_error "$session"; then
                ai_text="${RED}${DIM} [AI failed]${RESET}"
            else
                ai_text="${DIM} [AI loading...]${RESET}"
            fi
        fi

        if (( idx == SELECTED )); then
            buf_printf " ${REVERSE}${BOLD} > %-20s${RESET}%s" "$display_name" "$ai_text"
        else
            buf_printf "   %-20s%s" "$display_name" "$ai_text"
        fi
    done

    # Clear any leftover lines from previous render
    local clear_row=$(( start_row + max_items ))
    buf_cursor_to "$clear_row" 1
    buf_clear_line

    LIST_END=$(( start_row + max_items ))

    # Store offset for footer scroll indicator
    _LIST_OFFSET=$offset
    _LIST_MAX_ITEMS=$max_items
}

draw_separator() {
    local row="$1"
    buf_cursor_to "$row" 1
    buf_clear_line
    local separator=""
    local max_w=$(( TERM_COLS - 2 ))
    for (( c=0; c<max_w; c++ )); do separator+="─"; done
    buf_printf " ${DIM}${separator}${RESET}"
}

draw_preview() {
    local preview_start="$1"

    if [[ ${#SESSIONS[@]} -eq 0 ]]; then
        buf_cursor_to "$preview_start" 1
        buf_clear_line
        buf_printf " ${DIM}No tmux sessions found. Press [n] to create one.${RESET}"
        local r
        for (( r = preview_start + 1; r <= TERM_ROWS - 4; r++ )); do
            buf_cursor_to "$r" 1
            buf_clear_line
        done
        return
    fi

    local session="${SESSIONS[$SELECTED]}"
    buf_cursor_to "$preview_start" 1
    buf_clear_line
    buf_printf " ${BOLD}Preview${RESET} ${DIM}(${session}):${RESET}"

    local max_preview_lines=$(( TERM_ROWS - preview_start - 5 ))
    local line_num=0

    if (( max_preview_lines >= 1 )); then
        if (( max_preview_lines < 3 )); then max_preview_lines=3; fi
        if (( max_preview_lines > PREVIEW_LINES )); then max_preview_lines=$PREVIEW_LINES; fi

        local preview_content
        preview_content=$(capture_pane "$session" "$max_preview_lines")

        while IFS= read -r line; do
            line_num=$(( line_num + 1 ))
            if (( line_num > max_preview_lines )); then break; fi

            local row=$(( preview_start + line_num ))
            buf_cursor_to "$row" 1
            buf_clear_line
            # Truncate line to the smaller of terminal width and PREVIEW_MAX_COLS
            local max_len=$(( TERM_COLS - 2 ))
            if (( PREVIEW_MAX_COLS < max_len )); then max_len=$PREVIEW_MAX_COLS; fi
            buf_printf " ${GRAY}%s${RESET}" "${line:0:$max_len}"
        done <<< "$preview_content"
    fi

    # Clear remaining preview lines (also handles residual content after terminal shrink)
    local r
    for (( r = preview_start + line_num + 1; r <= TERM_ROWS - 4; r++ )); do
        buf_cursor_to "$r" 1
        buf_clear_line
    done
}

draw_footer() {
    # Separator at R-3
    draw_separator $(( TERM_ROWS - 3 ))

    # Tmux tips at R-2 (hidden on very small terminals)
    if (( TERM_ROWS >= 10 )); then
        buf_cursor_to $(( TERM_ROWS - 2 )) 1
        buf_clear_line
        local tips_text="tmux: C-b d detach  C-b c new  C-b n/p next/prev  C-b % hsplit  C-b \" vsplit"
        local truncated_tips
        truncated_tips=$(truncate_text "$tips_text" $(( TERM_COLS - 2 )))
        buf_printf " ${DIM}%s${RESET}" "$truncated_tips"
    fi

    # App keys at R-1
    local row=$(( TERM_ROWS - 1 ))
    buf_cursor_to "$row" 1
    buf_clear_line
    buf_printf " ${DIM}[Tab]${RESET} next  ${GREEN}[Enter]${RESET} open  ${BLUE}[n]${RESET} new  ${DIM}[f]${RESET} refresh  ${DIM}[q]${RESET} quit"

    # Scroll position indicator on right side
    if [[ ${#SESSIONS[@]} -gt 0 ]]; then
        local total=${#SESSIONS[@]}
        local pos_text="$(( SELECTED + 1 ))/${total}"
        local arrows=""
        if (( ${_LIST_OFFSET:-0} > 0 )); then arrows+="^"; fi
        if (( ${_LIST_OFFSET:-0} + ${_LIST_MAX_ITEMS:-0} < total )); then arrows+="v"; fi
        if [[ -n "$arrows" ]]; then pos_text="${arrows} ${pos_text}"; fi
        local col=$(( TERM_COLS - ${#pos_text} - 1 ))
        buf_cursor_to "$row" "$col"
        buf_printf "${DIM}%s${RESET}" "$pos_text"
    fi
}

draw_detail_footer() {
    # Separator at R-3
    draw_separator $(( TERM_ROWS - 3 ))

    # Tmux tips at R-2 (hidden on very small terminals)
    if (( TERM_ROWS >= 10 )); then
        buf_cursor_to $(( TERM_ROWS - 2 )) 1
        buf_clear_line
        local tips_text="tmux: C-b d detach  C-b c new  C-b n/p next/prev  C-b % hsplit  C-b \" vsplit"
        local truncated_tips
        truncated_tips=$(truncate_text "$tips_text" $(( TERM_COLS - 2 )))
        buf_printf " ${DIM}%s${RESET}" "$truncated_tips"
    fi

    # Detail keys at R-1
    local row=$(( TERM_ROWS - 1 ))
    buf_cursor_to "$row" 1
    buf_clear_line
    buf_printf " ${DIM}[Up/Down/Tab]${RESET} select  ${GREEN}[Enter]${RESET} confirm  ${GREEN}[a]${RESET}ttach ${BLUE}[r]${RESET}ename ${RED}[k]${RESET}ill  ${DIM}[ESC]${RESET} back"
}

render_list() {
    draw_header

    draw_session_list

    draw_separator $(( LIST_END + 1 ))
    draw_preview $(( LIST_END + 2 ))
    draw_footer
}

render_detail() {
    if [[ ${#SESSIONS[@]} -eq 0 ]]; then
        VIEW_MODE="list"
        render_list
        return
    fi

    local session="${SESSIONS[$SELECTED]}"

    # Header: session name + version
    local version_text="v${VERSION}"
    local name_max_detail=$(( TERM_COLS - ${#version_text} - 4 ))
    local display_session
    display_session=$(truncate_text "$session" "$name_max_detail")
    buf_cursor_to 1 1
    buf_clear_line
    buf_printf "${BOLD}${CYAN} %s${RESET}${DIM}  ${version_text}${RESET}" "$display_session"
    buf_cursor_to 2 1
    buf_clear_line
    local separator=""
    local max_w=$(( TERM_COLS - 2 ))
    for (( c=0; c<max_w; c++ )); do separator+="─"; done
    buf_printf " ${DIM}${separator}${RESET}"

    # Info line: session details + AI summary
    buf_cursor_to 3 1
    buf_clear_line
    local info
    info=$(get_session_info "$session")
    local info_detail="${info#*: }"
    buf_printf " ${BOLD}Info:${RESET} %s" "$info_detail"

    buf_cursor_to 4 1
    buf_clear_line
    local ai_text="${AI_SUMMARIES[$SELECTED]:-}"
    if [[ -n "$ai_text" ]]; then
        buf_printf " ${BOLD}AI:${RESET}   ${CYAN}%s${RESET}" "$ai_text"
    elif ai_enabled; then
        if ai_has_error "$session"; then
            buf_printf " ${BOLD}AI:${RESET}   ${RED}${DIM}(failed)${RESET}"
        else
            buf_printf " ${BOLD}AI:${RESET}   ${DIM}(loading...)${RESET}"
        fi
    fi

    # Separator
    draw_separator 5

    # Action menu
    local i
    for (( i=0; i<${#DETAIL_ACTIONS[@]}; i++ )); do
        local row=$(( 6 + i ))
        buf_cursor_to "$row" 1
        buf_clear_line
        local action="${DETAIL_ACTIONS[$i]}"
        local color=""
        case "$action" in
            kill) color="$RED" ;;
        esac
        if (( i == DETAIL_SELECTED )); then
            buf_printf " ${REVERSE}${BOLD}${color} > %-20s${RESET}" "$action"
        else
            buf_printf "   ${color}%-20s${RESET}" "$action"
        fi
    done

    # Clear remaining lines
    local r
    for (( r = 6 + ${#DETAIL_ACTIONS[@]}; r <= TERM_ROWS - 4; r++ )); do
        buf_cursor_to "$r" 1
        buf_clear_line
    done

    draw_detail_footer
}

render() {
    get_term_size
    load_ai_results || true

    _RENDER_BUF=""

    if [[ "$VIEW_MODE" == "detail" ]]; then
        render_detail
    else
        render_list
    fi

    buf_flush
}
