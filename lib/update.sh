#!/usr/bin/env bash
# lib/update.sh - Self-update from GitHub raw files

check_remote_version() {
    local content=""
    content=$(curl -fsSL "${GITHUB_RAW_BASE}/lib/constants.sh") || return 1

    local remote_version=""
    remote_version=$(printf "%s\n" "$content" | sed -n 's/^readonly VERSION="\([^"]*\)".*/\1/p' | head -1)
    [[ -n "$remote_version" ]] || return 1
    echo "$remote_version"
}

run_remote_installer() {
    local install_prefix="$1"
    local tmpdir=""
    tmpdir=$(mktemp -d) || return 1

    local installer="${tmpdir}/install.sh"
    if ! curl -fsSL "${GITHUB_RAW_BASE}/install.sh" -o "$installer"; then
        rm -rf "$tmpdir"
        return 1
    fi

    chmod +x "$installer"
    INSTALL_PREFIX="$install_prefix" bash "$installer"
    local status=$?
    rm -rf "$tmpdir"
    return $status
}

do_self_update() {
    command -v curl >/dev/null 2>&1 || {
        echo "Error: curl is required for --update" >&2
        return 1
    }

    local remote_version=""
    remote_version=$(check_remote_version) || {
        echo "Error: failed to check remote version" >&2
        return 1
    }

    if [[ "$remote_version" == "$VERSION" ]]; then
        echo "Already up to date (v${VERSION})"
        return 0
    fi

    # Resolve INSTALL_PREFIX from the real path of the current binary.
    # Installed layout: ${INSTALL_PREFIX}/share/tmux-manager/tmux-manager
    # So SCRIPT_DIR = ${INSTALL_PREFIX}/share/tmux-manager -> go up 2 levels.
    local install_prefix=""
    local current_bin=""
    current_bin=$(readlink -f "$(command -v tmux-manager 2>/dev/null || echo "${SCRIPT_DIR}/tmux-manager")")
    local current_dir=""
    current_dir=$(dirname "$current_bin")

    if [[ "$current_dir" == *"/share/tmux-manager" ]]; then
        # New layout: strip /share/tmux-manager
        install_prefix="${current_dir%/share/tmux-manager}"
    else
        # Development or legacy: use HOME/.local as default
        install_prefix="${HOME}/.local"
    fi

    echo "Updating tmux-manager v${VERSION} -> v${remote_version}"
    if run_remote_installer "$install_prefix"; then
        # Fix existing config file permissions if group/world-writable
        local config_file="${TMUX_MANAGER_CONFIG_FILE:-$(default_config_file)}"
        if [[ -f "$config_file" ]]; then
            local perms=""
            perms=$(stat -c '%a' "$config_file" 2>/dev/null \
                 || stat -f '%OLp' "$config_file" 2>/dev/null \
                 || echo "")
            if [[ -n "$perms" ]] && (( (8#${perms} & 8#022) != 0 )); then
                chmod 600 "$config_file"
                echo "Fixed config file permissions: ${config_file}"
            fi
        fi
        echo "Update complete."
        return 0
    fi

    echo "Update failed." >&2
    return 1
}
