#!/usr/bin/env bash
# lib/config.sh - User config loading

[[ -n "${_CONFIG_LOADED:-}" ]] && return
readonly _CONFIG_LOADED=1

default_config_file() {
    if [[ -n "${XDG_CONFIG_HOME:-}" ]]; then
        echo "${XDG_CONFIG_HOME}/tmux-manager/config.sh"
    else
        echo "${HOME}/.config/tmux-manager/config.sh"
    fi
}

normalize_bool() {
    case "${1,,}" in
        1|true|yes|on) echo "1" ;;
        *) echo "0" ;;
    esac
}

load_user_config() {
    TMUX_MANAGER_CONFIG_FILE="${TMUX_MANAGER_CONFIG_FILE:-$(default_config_file)}"
    TMUX_MANAGER_NEW_DEFAULT_DIR="${TMUX_MANAGER_NEW_DEFAULT_DIR:-}"
    TMUX_MANAGER_NEW_DEFAULT_CMD="${TMUX_MANAGER_NEW_DEFAULT_CMD:-}"
    TMUX_MANAGER_NEW_ASK_DIR="${TMUX_MANAGER_NEW_ASK_DIR:-0}"
    TMUX_MANAGER_NEW_ASK_CMD="${TMUX_MANAGER_NEW_ASK_CMD:-0}"

    if [[ -f "$TMUX_MANAGER_CONFIG_FILE" ]]; then
        # Refuse to load config files writable by group or others to prevent
        # malicious code injection via a tampered config file.
        local perms
        perms=$(stat -c '%a' "$TMUX_MANAGER_CONFIG_FILE" 2>/dev/null \
             || stat -f '%OLp' "$TMUX_MANAGER_CONFIG_FILE" 2>/dev/null \
             || echo "")
        # Check group-write (020) or world-write (002) bits via octal arithmetic
        if [[ -n "$perms" ]] && (( (8#${perms} & 8#022) != 0 )); then
            echo "Warning: Skipping config '$TMUX_MANAGER_CONFIG_FILE' (group/world-writable, mode $perms). Fix with: chmod go-w '$TMUX_MANAGER_CONFIG_FILE'" >&2
        else
            # shellcheck disable=SC1090
            source "$TMUX_MANAGER_CONFIG_FILE"
        fi
    fi

    TMUX_MANAGER_NEW_ASK_DIR=$(normalize_bool "${TMUX_MANAGER_NEW_ASK_DIR:-0}")
    TMUX_MANAGER_NEW_ASK_CMD=$(normalize_bool "${TMUX_MANAGER_NEW_ASK_CMD:-0}")

    if [[ -n "${TMUX_MANAGER_POLL_INTERVAL:-}" ]] && _is_positive_number "$TMUX_MANAGER_POLL_INTERVAL"; then
        KEY_POLL_INTERVAL="$TMUX_MANAGER_POLL_INTERVAL"
    fi
}

# ─── Config CLI helpers ─────────────────────────────────────────────

readonly -a CONFIG_KEYS=(NEW_DEFAULT_DIR NEW_DEFAULT_CMD NEW_ASK_DIR NEW_ASK_CMD POLL_INTERVAL)
readonly -a CONFIG_BOOL_KEYS=(NEW_ASK_DIR NEW_ASK_CMD)
readonly -a CONFIG_POSITIVE_NUM_KEYS=(POLL_INTERVAL)

_is_valid_config_key() {
    local key="$1" k
    for k in "${CONFIG_KEYS[@]}"; do
        [[ "$k" == "$key" ]] && return 0
    done
    return 1
}

_is_bool_key() {
    local key="$1" k
    for k in "${CONFIG_BOOL_KEYS[@]}"; do
        [[ "$k" == "$key" ]] && return 0
    done
    return 1
}

_is_positive_num_key() {
    local key="$1" k
    for k in "${CONFIG_POSITIVE_NUM_KEYS[@]}"; do
        [[ "$k" == "$key" ]] && return 0
    done
    return 1
}

_is_positive_number() {
    local val="$1"
    [[ "$val" =~ ^[0-9]*\.?[0-9]+$ ]] && (( $(echo "$val > 0" | bc -l 2>/dev/null || echo 0) ))
}

config_list() {
    TMUX_MANAGER_CONFIG_FILE="${TMUX_MANAGER_CONFIG_FILE:-$(default_config_file)}"
    # Reset to defaults before sourcing
    local TMUX_MANAGER_NEW_DEFAULT_DIR=""
    local TMUX_MANAGER_NEW_DEFAULT_CMD=""
    local TMUX_MANAGER_NEW_ASK_DIR="0"
    local TMUX_MANAGER_NEW_ASK_CMD="0"
    local TMUX_MANAGER_POLL_INTERVAL=""
    if [[ -f "$TMUX_MANAGER_CONFIG_FILE" ]]; then
        # shellcheck disable=SC1090
        source "$TMUX_MANAGER_CONFIG_FILE"
    fi
    local key
    for key in "${CONFIG_KEYS[@]}"; do
        local var="TMUX_MANAGER_${key}"
        echo "${key}=${!var}"
    done
}

config_get() {
    local key="$1"
    if ! _is_valid_config_key "$key"; then
        echo "Invalid config key: ${key}" >&2
        return 1
    fi
    TMUX_MANAGER_CONFIG_FILE="${TMUX_MANAGER_CONFIG_FILE:-$(default_config_file)}"
    local TMUX_MANAGER_NEW_DEFAULT_DIR=""
    local TMUX_MANAGER_NEW_DEFAULT_CMD=""
    local TMUX_MANAGER_NEW_ASK_DIR="0"
    local TMUX_MANAGER_NEW_ASK_CMD="0"
    local TMUX_MANAGER_POLL_INTERVAL=""
    if [[ -f "$TMUX_MANAGER_CONFIG_FILE" ]]; then
        # shellcheck disable=SC1090
        source "$TMUX_MANAGER_CONFIG_FILE"
    fi
    local var="TMUX_MANAGER_${key}"
    echo "${!var}"
}

config_set() {
    local key="$1" value="$2"
    if ! _is_valid_config_key "$key"; then
        echo "Invalid config key: ${key}" >&2
        return 1
    fi
    if _is_bool_key "$key"; then
        value=$(normalize_bool "$value")
    elif _is_positive_num_key "$key"; then
        if ! _is_positive_number "$value"; then
            echo "Invalid value for ${key}: must be a positive number" >&2
            return 1
        fi
    fi
    TMUX_MANAGER_CONFIG_FILE="${TMUX_MANAGER_CONFIG_FILE:-$(default_config_file)}"
    local config_dir
    config_dir="$(dirname "$TMUX_MANAGER_CONFIG_FILE")"
    if [[ ! -d "$config_dir" ]]; then
        mkdir -p "$config_dir"
        chmod 700 "$config_dir"
    fi
    local full_key="TMUX_MANAGER_${key}"
    local line="${full_key}=\"${value}\""
    local is_new=false
    [[ ! -f "$TMUX_MANAGER_CONFIG_FILE" ]] && is_new=true

    if [[ -f "$TMUX_MANAGER_CONFIG_FILE" ]] && grep -q "^${full_key}=" "$TMUX_MANAGER_CONFIG_FILE"; then
        sed -i "s|^${full_key}=.*|${line}|" "$TMUX_MANAGER_CONFIG_FILE"
    else
        echo "$line" >> "$TMUX_MANAGER_CONFIG_FILE"
    fi

    if $is_new; then
        chmod 600 "$TMUX_MANAGER_CONFIG_FILE"
    fi
}
