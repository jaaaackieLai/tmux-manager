#!/usr/bin/env bats
# tests/test_install_rust.bats - Tests for install.sh (Rust download entry)

load 'test_helper'
bats_require_minimum_version 1.5.0

setup() {
    TEST_PREFIX="$(mktemp -d)"
    TEST_WORK="$(mktemp -d)"
}

teardown() {
    rm -rf "$TEST_PREFIX" "$TEST_WORK"
}

# 依 Bash 版（v1.x）installer 的版面建立既有安裝：share 內放主程式與 lib，bin 是絕對 symlink。
make_v1_install() {
    local share="${TEST_PREFIX}/share/tmux-manager" lib
    mkdir -p "${share}/lib" "${TEST_PREFIX}/bin"
    printf '#!/usr/bin/env bash\necho v1\n' > "${share}/tmux-manager"
    chmod +x "${share}/tmux-manager"
    for lib in actions ai config constants input render sessions update utils; do
        echo "# v1 ${lib}" > "${share}/lib/${lib}.sh"
    done
    ln -s "${share}/tmux-manager" "${TEST_PREFIX}/bin/tmux-manager"
}

@test "--uninstall removes a legacy Bash install instead of launching it" {
    make_v1_install
    touch "${TEST_PREFIX}/bin/other-tool"

    run with_timeout 10 env INSTALL_PREFIX="$TEST_PREFIX" bash "$REPO_ROOT/install.sh" --uninstall </dev/null

    [ "$status" -eq 0 ]
    [ ! -e "${TEST_PREFIX}/bin/tmux-manager" ]
    [ ! -L "${TEST_PREFIX}/bin/tmux-manager" ]
    [ ! -e "${TEST_PREFIX}/share/tmux-manager" ]
    [ -f "${TEST_PREFIX}/bin/other-tool" ]
}

@test "--uninstall --purge also removes legacy config but keeps unknown files" {
    make_v1_install
    local config_dir="${TEST_WORK}/xdg/tmux-manager"
    mkdir -p "$config_dir"
    touch "${config_dir}/config.sh" "${config_dir}/prompts.toml" "${config_dir}/notes.md"

    run with_timeout 10 env INSTALL_PREFIX="$TEST_PREFIX" XDG_CONFIG_HOME="${TEST_WORK}/xdg" \
        bash "$REPO_ROOT/install.sh" --uninstall --purge </dev/null

    [ "$status" -eq 0 ]
    [ ! -e "${config_dir}/config.sh" ]
    [ ! -e "${config_dir}/prompts.toml" ]
    [ -f "${config_dir}/notes.md" ]
}

@test "--uninstall --purge follows the Rust config path rules for a v1 config.sh override" {
    make_v1_install
    local custom="${TEST_WORK}/custom"
    mkdir -p "$custom"
    touch "${custom}/config.sh" "${custom}/config.toml"

    run env INSTALL_PREFIX="$TEST_PREFIX" TMUX_MANAGER_CONFIG_FILE="${custom}/config.sh" \
        bash "$REPO_ROOT/install.sh" --uninstall --purge </dev/null

    [ "$status" -eq 0 ]
    # 與 Rust 相同：*.sh 對應到同名 *.toml；不在 tmux-manager 資料夾內的 config.sh 屬於使用者，保留。
    [ ! -e "${custom}/config.toml" ]
    [ -f "${custom}/config.sh" ]
}

@test "--uninstall without an install reports it clearly" {
    run env INSTALL_PREFIX="$TEST_PREFIX" bash "$REPO_ROOT/install.sh" --uninstall

    [ "$status" -ne 0 ]
    [[ "$output" == *"找不到 tmux-manager 安裝"* ]]
}

# 假 curl：記錄下載網址並以失敗結束，避免真的連線。
fake_failing_curl() {
    mkdir -p "${TEST_WORK}/bin"
    cat > "${TEST_WORK}/bin/curl" <<'EOF'
#!/usr/bin/env bash
echo "$*" >> "${CURL_LOG}.args"
for arg in "$@"; do [[ "$arg" == https://* ]] && echo "$arg" >> "$CURL_LOG"; done
exit 22
EOF
    chmod +x "${TEST_WORK}/bin/curl"
}

# 模擬 `curl ... | bash`：以 stdin 執行、沒有腳本檔，cwd 在 TEST_WORK。
piped_install() {
    (cd "$TEST_WORK" && env PATH="${TEST_WORK}/bin:$PATH" CURL_LOG="${TEST_WORK}/urls" \
        INSTALL_PREFIX="$TEST_PREFIX" "$@" bash < "$REPO_ROOT/install.sh")
}

remote_install_urls() {
    fake_failing_curl
    piped_install "$@" || true
    cat "${TEST_WORK}/urls"
}

@test "remote install downloads the latest release by default" {
    run remote_install_urls

    [[ "$output" == *"/releases/latest/download/tmux-manager-"* ]]
}

@test "TMUX_MANAGER_VERSION pins the remote install to that release" {
    run remote_install_urls TMUX_MANAGER_VERSION=v9.9.9

    [[ "$output" == *"/releases/download/v9.9.9/tmux-manager-"* ]]
}

fake_uname() {
    mkdir -p "${TEST_WORK}/bin"
    printf '#!/usr/bin/env bash\n[[ "$1" == -s ]] && echo %s || echo %s\n' "$1" "$2" > "${TEST_WORK}/bin/uname"
    chmod +x "${TEST_WORK}/bin/uname"
}

@test "Linux x86_64 downloads the static musl artifact" {
    fake_uname Linux x86_64
    run remote_install_urls

    [[ "$output" == *"/tmux-manager-x86_64-unknown-linux-musl"* ]]
}

@test "Linux aarch64 downloads its own release artifact" {
    fake_uname Linux aarch64
    run remote_install_urls

    [[ "$output" == *"/tmux-manager-aarch64-unknown-linux-musl"* ]]
}

@test "a failed download explains it and keeps the existing install" {
    mkdir -p "${TEST_PREFIX}/bin"
    echo 'existing' > "${TEST_PREFIX}/bin/tmux-manager"
    fake_failing_curl
    run piped_install

    [ "$status" -ne 0 ]
    [[ "$output" == *"下載失敗"* ]]
    [[ "$output" == *"現有安裝保留"* ]]
    [ "$(cat "${TEST_PREFIX}/bin/tmux-manager")" = existing ]
}

fake_binary_and_sudo() {
    mkdir -p "${TEST_WORK}/bin"
    printf '#!/usr/bin/env bash\necho "binary $*" >> "%s/log"\n' "$TEST_WORK" > "${TEST_WORK}/fake-binary"
    printf '#!/usr/bin/env bash\necho "sudo $*" >> "%s/log"\n' "$TEST_WORK" > "${TEST_WORK}/bin/sudo"
    chmod +x "${TEST_WORK}/fake-binary" "${TEST_WORK}/bin/sudo"
}

@test "an unwritable prefix installs through sudo" {
    [[ "$(id -u)" -ne 0 ]] || skip "root 可寫入任何路徑"
    fake_binary_and_sudo
    local prefix="${TEST_PREFIX}/locked"
    mkdir -p "$prefix"
    chmod 555 "$prefix"

    run env PATH="${TEST_WORK}/bin:$PATH" INSTALL_PREFIX="$prefix" \
        TMUX_MANAGER_BINARY="${TEST_WORK}/fake-binary" bash "$REPO_ROOT/install.sh"
    chmod 755 "$prefix"

    [ "$status" -eq 0 ]
    grep -q "^sudo ${TEST_WORK}/fake-binary install --prefix ${prefix}$" "${TEST_WORK}/log"
}

@test "a writable prefix installs without sudo" {
    fake_binary_and_sudo

    run env PATH="${TEST_WORK}/bin:$PATH" INSTALL_PREFIX="${TEST_PREFIX}/new" \
        TMUX_MANAGER_BINARY="${TEST_WORK}/fake-binary" bash "$REPO_ROOT/install.sh"

    [ "$status" -eq 0 ]
    grep -q "^binary install --prefix ${TEST_PREFIX}/new$" "${TEST_WORK}/log"
    run ! grep -q '^sudo' "${TEST_WORK}/log"
}

@test "piped install never picks up a local build from the current directory" {
    mkdir -p "${TEST_WORK}/target/release"
    printf '#!/usr/bin/env bash\necho local >> "%s/local-used"\n' "$TEST_WORK" > "${TEST_WORK}/target/release/tmux-manager"
    chmod +x "${TEST_WORK}/target/release/tmux-manager"

    run remote_install_urls

    [ ! -e "${TEST_WORK}/local-used" ]
    [[ "$output" == *"/releases/latest/download/tmux-manager-"* ]]
}

@test "slow links are not cut off by a total download time limit" {
    remote_install_urls >/dev/null

    # v1 的 --update 經由這裡升級：慢速網路要能下載完，只在停滯時中止。
    run ! grep -q -- '--max-time' "${TEST_WORK}/urls.args"
    grep -q -- '--speed-time' "${TEST_WORK}/urls.args"
}

@test "a sudo uninstall --purge still purges the invoking user's config" {
    [[ "$(id -u)" -ne 0 ]] || skip "root 可寫入任何路徑"
    fake_binary_and_sudo
    mkdir -p "${TEST_PREFIX}/bin" "${TEST_PREFIX}/share/tmux-manager"
    cp "${TEST_WORK}/fake-binary" "${TEST_PREFIX}/bin/tmux-manager"
    chmod 555 "${TEST_PREFIX}/bin"

    run env PATH="${TEST_WORK}/bin:$PATH" INSTALL_PREFIX="$TEST_PREFIX" XDG_CONFIG_HOME="${TEST_WORK}/xdg" \
        bash "$REPO_ROOT/install.sh" --uninstall --purge
    chmod 755 "${TEST_PREFIX}/bin"

    [ "$status" -eq 0 ]
    # sudo 會重設 HOME/XDG 等變數；設定檔路徑必須在提權前解析並明確傳入。
    grep -q -- "--config-file ${TEST_WORK}/xdg/tmux-manager/config.toml" "${TEST_WORK}/log"
}
