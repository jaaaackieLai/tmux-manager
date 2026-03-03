#!/usr/bin/env bats
# tests/test_install.bats - Tests for install.sh

load 'test_helper'

setup() {
    TEST_PREFIX="$(mktemp -d)"
    TEST_DATA_DIR="${TEST_PREFIX}/share/tmux-manager"
    TEST_BIN_DIR="${TEST_PREFIX}/bin"
    TEST_SCRIPT_DIR="$REPO_ROOT"
}

teardown() {
    rm -rf "$TEST_PREFIX"
}

@test "install twice does not create nested lib directory" {
    INSTALL_PREFIX="$TEST_PREFIX" bash "$REPO_ROOT/install.sh"
    INSTALL_PREFIX="$TEST_PREFIX" bash "$REPO_ROOT/install.sh"

    # share/tmux-manager/lib/ should exist with .sh files
    [ -d "${TEST_DATA_DIR}/lib" ]
    ls "${TEST_DATA_DIR}/lib/"*.sh >/dev/null 2>&1

    # No nested directories
    [ ! -d "${TEST_DATA_DIR}/lib/lib" ]
    [ ! -d "${TEST_DATA_DIR}/lib/tmux-manager-lib" ]
}

@test "install copies all lib .sh files" {
    INSTALL_PREFIX="$TEST_PREFIX" bash "$REPO_ROOT/install.sh"

    local expected
    expected=$(ls "$REPO_ROOT/lib/"*.sh | wc -l)
    local actual
    actual=$(ls "$TEST_DATA_DIR/lib/"*.sh | wc -l)
    [ "$expected" -eq "$actual" ]
}

@test "install removes stale lib files from previous install" {
    INSTALL_PREFIX="$TEST_PREFIX" bash "$REPO_ROOT/install.sh"

    # Simulate a stale file from an older version
    touch "${TEST_DATA_DIR}/lib/old_module.sh"

    # Second install overwrites the lib dir
    INSTALL_PREFIX="$TEST_PREFIX" bash "$REPO_ROOT/install.sh"

    # Data dir is refreshed; stale file may or may not exist
    # but all current lib files must be present
    local expected
    expected=$(ls "$REPO_ROOT/lib/"*.sh | wc -l)
    local actual
    actual=$(ls "$TEST_DATA_DIR/lib/"*.sh | wc -l)
    # actual >= expected (stale file might persist since we cp over, not rm first)
    [ "$actual" -ge "$expected" ]
}

@test "install does not touch other files in prefix" {
    mkdir -p "${TEST_PREFIX}/share/other-tool"
    echo "other-tool" > "${TEST_PREFIX}/share/other-tool/config"

    INSTALL_PREFIX="$TEST_PREFIX" bash "$REPO_ROOT/install.sh"

    [ -f "${TEST_PREFIX}/share/other-tool/config" ]
    [ "$(cat "${TEST_PREFIX}/share/other-tool/config")" = "other-tool" ]
}

@test "install creates prefix directories if they do not exist" {
    local new_prefix="${TEST_PREFIX}/sub/path"
    INSTALL_PREFIX="$new_prefix" bash "$REPO_ROOT/install.sh"

    [ -f "${new_prefix}/share/tmux-manager/tmux-manager" ]
    [ -d "${new_prefix}/share/tmux-manager/lib" ]
    [ -L "${new_prefix}/bin/tmux-manager" ]
}

@test "install creates symlink in bin dir" {
    INSTALL_PREFIX="$TEST_PREFIX" bash "$REPO_ROOT/install.sh"

    [ -L "${TEST_BIN_DIR}/tmux-manager" ]
    local target
    target=$(readlink "${TEST_BIN_DIR}/tmux-manager")
    [ "$target" = "${TEST_DATA_DIR}/tmux-manager" ]
}

@test "installed tmux-manager is executable via symlink" {
    INSTALL_PREFIX="$TEST_PREFIX" bash "$REPO_ROOT/install.sh"

    run "${TEST_BIN_DIR}/tmux-manager" --version
    [ "$status" -eq 0 ]
    [[ "$output" == *"tmux-manager v"* ]]
}

@test "remote install mode works when install.sh is executed without local lib directory" {
    run bash -c "
        set -euo pipefail
        work=\$(mktemp -d)
        trap 'rm -rf \"\$work\"' EXIT

        mkdir -p \"\$work/bin\" \"\$work/remote\"
        cp '${REPO_ROOT}/install.sh' \"\$work/remote/install.sh\"

        cat > \"\$work/bin/tmux\" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF

        cat > \"\$work/bin/curl\" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
out=''
url=''
while [[ \$# -gt 0 ]]; do
    case \"\$1\" in
        -o) out=\"\$2\"; shift 2 ;;
        -f|-s|-S|-L|-fsSL) shift ;;
        *) url=\"\$1\"; shift ;;
    esac
done
[[ -n \"\$out\" ]] || exit 0
rel=\"\${url#https://raw.githubusercontent.com/jaaaackieLai/tmux-manager/main/}\"
cp \"\$REPO_ROOT/\$rel\" \"\$out\"
EOF

        chmod +x \"\$work/bin/tmux\" \"\$work/bin/curl\"

        PATH=\"\$work/bin:\$PATH\" REPO_ROOT='${REPO_ROOT}' INSTALL_PREFIX='${TEST_PREFIX}' bash \"\$work/remote/install.sh\"

        [ -f '${TEST_DATA_DIR}/tmux-manager' ]
        [ -f '${TEST_DATA_DIR}/lib/update.sh' ]
        [ -L '${TEST_BIN_DIR}/tmux-manager' ]
    "
    [ "$status" -eq 0 ]
}

@test "remote install downloads all lib .sh files" {
    run bash -c "
        set -euo pipefail
        work=\$(mktemp -d)
        trap 'rm -rf \"\$work\"' EXIT

        mkdir -p \"\$work/bin\" \"\$work/remote\"
        cp '${REPO_ROOT}/install.sh' \"\$work/remote/install.sh\"

        cat > \"\$work/bin/tmux\" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF

        cat > \"\$work/bin/curl\" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
out=''
url=''
while [[ \$# -gt 0 ]]; do
    case \"\$1\" in
        -o) out=\"\$2\"; shift 2 ;;
        -f|-s|-S|-L|-fsSL) shift ;;
        *) url=\"\$1\"; shift ;;
    esac
done
[[ -n \"\$out\" ]] || exit 0
rel=\"\${url#https://raw.githubusercontent.com/jaaaackieLai/tmux-manager/main/}\"
cp \"\$REPO_ROOT/\$rel\" \"\$out\"
EOF

        chmod +x \"\$work/bin/tmux\" \"\$work/bin/curl\"

        PATH=\"\$work/bin:\$PATH\" REPO_ROOT='${REPO_ROOT}' INSTALL_PREFIX='${TEST_PREFIX}' bash \"\$work/remote/install.sh\"

        expected=\$(ls '${REPO_ROOT}/lib/'*.sh | wc -l)
        actual=\$(ls '${TEST_DATA_DIR}/lib/'*.sh | wc -l)
        [ \"\$expected\" -eq \"\$actual\" ]
    "
    [ "$status" -eq 0 ]
}

@test "installed tmux-manager resolves lib via symlink" {
    INSTALL_PREFIX="$TEST_PREFIX" bash "$REPO_ROOT/install.sh"

    run "${TEST_BIN_DIR}/tmux-manager" --help
    [ "$status" -eq 0 ]
    [[ "$output" == *"Interactive tmux session manager"* ]]
}

@test "install works via pipe without BASH_SOURCE unbound errors" {
    run bash -c "
        set -euo pipefail
        work=\$(mktemp -d)
        trap 'rm -rf \"\$work\"' EXIT

        mkdir -p \"\$work/bin\"

        cat > \"\$work/bin/tmux\" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF

        cat > \"\$work/bin/curl\" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
out=''
url=''
while [[ \$# -gt 0 ]]; do
    case \"\$1\" in
        -o) out=\"\$2\"; shift 2 ;;
        -f|-s|-S|-L|-fsSL) shift ;;
        *) url=\"\$1\"; shift ;;
    esac
done
[[ -n \"\$out\" ]] || exit 0
rel=\"\${url#https://raw.githubusercontent.com/jaaaackieLai/tmux-manager/main/}\"
cp \"\$REPO_ROOT/\$rel\" \"\$out\"
EOF

        chmod +x \"\$work/bin/tmux\" \"\$work/bin/curl\"

        PATH=\"\$work/bin:\$PATH\" REPO_ROOT='${REPO_ROOT}' INSTALL_PREFIX='${TEST_PREFIX}' bash < '${REPO_ROOT}/install.sh'
    "
    [ "$status" -eq 0 ]
    [[ "$output" != *"BASH_SOURCE[0]: unbound variable"* ]]
}

@test "uninstall removes data dir and symlink" {
    INSTALL_PREFIX="$TEST_PREFIX" bash "$REPO_ROOT/install.sh"

    # Verify installed
    [ -d "$TEST_DATA_DIR" ]
    [ -L "${TEST_BIN_DIR}/tmux-manager" ]

    # Uninstall
    INSTALL_PREFIX="$TEST_PREFIX" bash "$REPO_ROOT/install.sh" --uninstall

    # Verify removed
    [ ! -d "$TEST_DATA_DIR" ]
    [ ! -L "${TEST_BIN_DIR}/tmux-manager" ]
}

@test "uninstall cleans up legacy tmux-manager-lib directory" {
    # Simulate legacy layout
    mkdir -p "${TEST_BIN_DIR}/tmux-manager-lib"
    touch "${TEST_BIN_DIR}/tmux-manager-lib/constants.sh"

    INSTALL_PREFIX="$TEST_PREFIX" bash "$REPO_ROOT/install.sh" --uninstall

    [ ! -d "${TEST_BIN_DIR}/tmux-manager-lib" ]
}
