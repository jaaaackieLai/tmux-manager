#!/usr/bin/env bats
# tests/test_actions.bats - Tests for lib/actions.sh

load 'test_helper'

@test "action_rename uses AI suggestion when pressing Enter" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'

        SESSIONS=(alpha)
        SELECTED=0
        AI_NAMES=('ai-name')
        TERM_ROWS=20
        SAVED_TTY=''

        cursor_show() { :; }
        cursor_hide() { :; }
        cursor_to() { :; }
        clear_line() { :; }
        refresh_sessions() { :; }
        render() { :; }

        tmux() {
            if [[ \"\$1\" == 'rename-session' ]]; then
                echo \"RENAMED:\${3}:\${4}\"
            fi
        }

        printf '\n' | action_rename
    "
    [ "$status" -eq 0 ]
    [[ "$output" == *"RENAMED:alpha:ai-name"* ]]
}

@test "action_rename supports custom input when pressing e" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'

        SESSIONS=(alpha)
        SELECTED=0
        AI_NAMES=('ai-name')
        TERM_ROWS=20
        SAVED_TTY=''

        cursor_show() { :; }
        cursor_hide() { :; }
        cursor_to() { :; }
        clear_line() { :; }
        refresh_sessions() { :; }
        render() { :; }

        tmux() {
            if [[ \"\$1\" == 'rename-session' ]]; then
                echo \"RENAMED:\${3}:\${4}\"
            fi
        }

        printf 'emy-custom-name\n' | action_rename
    "
    [ "$status" -eq 0 ]
    [[ "$output" == *"RENAMED:alpha:my-custom-name"* ]]
}

@test "action_rename cancels on ESC without renaming" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'

        SESSIONS=(alpha)
        SELECTED=0
        AI_NAMES=('ai-name')
        TERM_ROWS=20
        SAVED_TTY=''

        cursor_show() { :; }
        cursor_hide() { :; }
        cursor_to() { :; }
        clear_line() { :; }
        refresh_sessions() { :; }
        render() { :; }

        tmux() {
            if [[ \"\$1\" == 'rename-session' ]]; then
                echo \"RENAMED:\${3}:\${4}\"
            fi
        }

        printf '\033' | action_rename
    "
    [ "$status" -eq 0 ]
    [[ "$output" != *"RENAMED:"* ]]
}

@test "action_new applies default workdir and init command" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'

        TERM_ROWS=20
        SAVED_TTY=''
        TMUX_MANAGER_NEW_DEFAULT_DIR='/tmp/workspace'
        TMUX_MANAGER_NEW_DEFAULT_CMD='source .venv/bin/activate'
        TMUX_MANAGER_NEW_ASK_DIR=0
        TMUX_MANAGER_NEW_ASK_CMD=0

        cursor_show() { :; }
        cursor_hide() { :; }
        cursor_to() { :; }
        clear_line() { :; }
        refresh_sessions() { :; }
        start_ai_summaries() { :; }
        render() { :; }

        tmux() {
            if [[ \"\$1\" == 'new-session' ]]; then
                echo \"NEW:\$*\"
                return 0
            elif [[ \"\$1\" == 'send-keys' ]]; then
                echo \"SEND:\$*\"
                return 0
            fi
        }

        printf 'proj\n' | action_new
    "
    [ "$status" -eq 0 ]
    [[ "$output" == *"NEW:new-session -d -s proj -c /tmp/workspace"* ]]
    [[ "$output" == *"SEND:send-keys -t proj source .venv/bin/activate C-m"* ]]
}

@test "action_new prompts can override default workdir and init command" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'

        TERM_ROWS=20
        SAVED_TTY=''
        TMUX_MANAGER_NEW_DEFAULT_DIR='/tmp/default'
        TMUX_MANAGER_NEW_DEFAULT_CMD='npm test'
        TMUX_MANAGER_NEW_ASK_DIR=1
        TMUX_MANAGER_NEW_ASK_CMD=1

        cursor_show() { :; }
        cursor_hide() { :; }
        cursor_to() { :; }
        clear_line() { :; }
        refresh_sessions() { :; }
        start_ai_summaries() { :; }
        render() { :; }

        tmux() {
            if [[ \"\$1\" == 'new-session' ]]; then
                echo \"NEW:\$*\"
                return 0
            elif [[ \"\$1\" == 'send-keys' ]]; then
                echo \"SEND:\$*\"
                return 0
            fi
        }

        printf 'proj\n/tmp/override\npnpm dev\n' | action_new
    "
    [ "$status" -eq 0 ]
    [[ "$output" == *"NEW:new-session -d -s proj -c /tmp/override"* ]]
    [[ "$output" == *"SEND:send-keys -t proj pnpm dev C-m"* ]]
}

@test "action_attach does not restart AI summaries after detach" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'

        SESSIONS=(alpha)
        SELECTED=0
        SAVED_TTY=''

        cursor_show() { :; }
        cursor_hide() { :; }
        clear_screen() { :; }
        refresh_sessions() { :; }
        render() { :; }
        start_ai_summaries() { echo 'AI_RESTARTED'; }

        tmux() {
            if [[ \"\$1\" == 'attach' ]]; then
                return 0
            fi
        }

        action_attach
    "
    [ "$status" -eq 0 ]
    [[ "$output" != *"AI_RESTARTED"* ]]
}

@test "action_new attaches to session after creation" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'

        TERM_ROWS=20
        SAVED_TTY=''
        TMUX_MANAGER_NEW_DEFAULT_DIR=''
        TMUX_MANAGER_NEW_DEFAULT_CMD=''
        TMUX_MANAGER_NEW_ASK_DIR=0
        TMUX_MANAGER_NEW_ASK_CMD=0

        cursor_show() { :; }
        cursor_hide() { :; }
        cursor_to() { :; }
        clear_line() { :; }
        clear_screen() { :; }
        start_ai_summaries() { :; }
        render() { :; }

        _created=0
        SESSIONS=()
        refresh_sessions() {
            if [[ \$_created -eq 1 ]]; then
                SESSIONS=(myproj)
            fi
        }

        tmux() {
            if [[ \"\$1\" == 'new-session' ]]; then
                echo \"NEW:\$*\"
                _created=1
                return 0
            elif [[ \"\$1\" == 'attach' ]]; then
                echo \"ATTACH:\$*\"
                return 0
            fi
        }

        printf 'myproj\n' | action_new
    "
    [ "$status" -eq 0 ]
    [[ "$output" == *"NEW:new-session -d -s myproj"* ]]
    [[ "$output" == *"ATTACH:attach -t myproj"* ]]
}

@test "action_new allows '-' to disable default init command" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        source '${LIB_DIR}/render.sh'
        source '${LIB_DIR}/actions.sh'

        TERM_ROWS=20
        SAVED_TTY=''
        TMUX_MANAGER_NEW_DEFAULT_DIR=''
        TMUX_MANAGER_NEW_DEFAULT_CMD='source .venv/bin/activate'
        TMUX_MANAGER_NEW_ASK_DIR=0
        TMUX_MANAGER_NEW_ASK_CMD=1

        cursor_show() { :; }
        cursor_hide() { :; }
        cursor_to() { :; }
        clear_line() { :; }
        refresh_sessions() { :; }
        start_ai_summaries() { :; }
        render() { :; }

        tmux() {
            if [[ \"\$1\" == 'new-session' ]]; then
                echo \"NEW:\$*\"
                return 0
            elif [[ \"\$1\" == 'send-keys' ]]; then
                echo \"SEND:\$*\"
                return 0
            fi
        }

        printf 'proj\n-\n' | action_new
    "
    [ "$status" -eq 0 ]
    [[ "$output" == *"NEW:new-session -d -s proj"* ]]
    [[ "$output" != *"SEND:send-keys"* ]]
}
