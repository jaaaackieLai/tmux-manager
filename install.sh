#!/usr/bin/env bash
# 選用的下載入口；安裝、更新及移除操作交由 Rust binary。
set -euo pipefail
prefix="${INSTALL_PREFIX:-${HOME}/.local}"
bin="${prefix}/bin/tmux-manager"
share="${prefix}/share/tmux-manager"
# Bash 版（v1.x）不認得 Rust 子指令；移除時由這裡刪除它安裝的已知檔名。
remove_legacy() {
    local dir lib
    rm -f "$bin" "${share}/tmux-manager"
    for dir in "${share}/lib" "${prefix}/bin/tmux-manager-lib"; do
        for lib in actions ai config constants input render sessions update utils; do
            rm -f "${dir}/${lib}.sh"
        done
        rmdir "$dir" 2>/dev/null || true
    done
    rmdir "$share" 2>/dev/null || true
}
# 與 Rust `Config::resolve_path` 相同：空值視為未設定；指向 Bash 版 *.sh 時對應同名 *.toml。
user_config_path() {
    local config="${TMUX_MANAGER_CONFIG_FILE:-${XDG_CONFIG_HOME:-${HOME}/.config}/tmux-manager/config.toml}"
    [[ "$config" == *.sh ]] && config="${config%.sh}.toml"
    printf '%s\n' "$config"
}
# 與 Rust `uninstall --purge` 相同：只刪精確檔名，資料夾清空才移除。
purge_config() {
    local config dir
    config="$(user_config_path)"
    dir="$(dirname "$config")"
    rm -f "$config" "${config%.*}.toml.lock" "${dir}/prompts.toml" "${dir}/prompts.toml.lock"
    if [[ "$(basename "$dir")" == tmux-manager ]]; then
        rm -f "${dir}/config.sh"
    fi
    rmdir "$dir" 2>/dev/null || true
}
# 安裝目標（或其最近的既有上層資料夾）不可寫入時需要 sudo，例如 /usr/local。
needs_sudo() {
    local dir
    for dir in "${prefix}/bin" "$share"; do
        while [[ ! -e "$dir" ]]; do dir="$(dirname "$dir")"; done
        [[ -w "$dir" ]] || return 0
    done
    return 1
}
run_installer() {
    if needs_sudo; then
        echo "需要 sudo 才能寫入 ${prefix}" >&2
        sudo "$@"
    else
        "$@"
    fi
}
# 不限制總時間（慢速網路也要能完成升級）；連線逾時或 30 秒內幾乎沒有進度才中止。
download() {
    curl -fsSL --connect-timeout 15 --speed-limit 1024 --speed-time 30 "$1" -o "$2" || {
        echo "下載失敗：$1（release 尚未發布或網路異常）；現有安裝保留" >&2
        exit 1
    }
}
if [[ "${1:-}" == "--uninstall" ]]; then
    if [[ -L "$bin" && "$(readlink "$bin")" == */share/tmux-manager/tmux-manager ]] \
        || [[ ! -e "$bin" && -f "${share}/tmux-manager" ]]; then
        remove_legacy
        if [[ "${2:-}" == "--purge" ]]; then
            purge_config
        fi
        echo '已移除 Bash 版 tmux-manager'
        exit 0
    fi
    [[ -e "$bin" ]] || { echo "找不到 tmux-manager 安裝：${bin}" >&2; exit 1; }
    # sudo 會重設 HOME/XDG 等變數：在提權前解析使用者的設定檔路徑並明確傳入。
    run_installer "$bin" --config-file "$(user_config_path)" uninstall --prefix "$prefix" "${@:2}"
    exit 0
fi
if [[ -n "${TMUX_MANAGER_BINARY:-}" ]]; then
    run_installer "$TMUX_MANAGER_BINARY" install --prefix "$prefix"
    exit 0
fi
# 只有從 clone 下來的檔案執行時才使用旁邊的本機建置；`curl | bash` 沒有腳本檔，
# 不可退回目前目錄，否則會安裝未驗證的 binary。
if [[ -f "${BASH_SOURCE[0]:-}" ]]; then
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    if [[ -x "${script_dir}/target/release/tmux-manager" ]]; then
        run_installer "${script_dir}/target/release/tmux-manager" install --prefix "$prefix"
        exit 0
    fi
fi
case "$(uname -s)/$(uname -m)" in
    Linux/x86_64) target=x86_64-unknown-linux-musl ;;
    Linux/aarch64|Linux/arm64) target=aarch64-unknown-linux-musl ;;
    Darwin/x86_64) target=x86_64-apple-darwin ;;
    Darwin/arm64) target=aarch64-apple-darwin ;;
    *) echo '此平台請使用 cargo install --path .' >&2; exit 1 ;;
esac
artifact="tmux-manager-${target}"
releases="https://github.com/jaaaackieLai/tmux-manager/releases"
if [[ -n "${TMUX_MANAGER_VERSION:-}" ]]; then
    base="${releases}/download/${TMUX_MANAGER_VERSION}"
else
    base="${releases}/latest/download"
fi
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
download "${base}/${artifact}" "${work}/${artifact}"
download "${base}/${artifact}.sha256" "${work}/checksum"
read -r expected declared < "${work}/checksum"
[[ "$expected" =~ ^[a-fA-F0-9]{64}$ && "${declared#\*}" == "$artifact" ]] || { echo 'checksum 格式或 artifact 名稱不符' >&2; exit 1; }
if command -v sha256sum >/dev/null 2>&1; then
    actual="$(sha256sum "${work}/${artifact}")"
else
    actual="$(shasum -a 256 "${work}/${artifact}")"
fi
expected="$(printf '%s' "$expected" | tr '[:upper:]' '[:lower:]')"
[[ "${actual%% *}" == "$expected" ]] || { echo 'checksum 不符；現有安裝保留' >&2; exit 1; }
chmod +x "${work}/${artifact}"
run_installer "${work}/${artifact}" install --prefix "$prefix"
