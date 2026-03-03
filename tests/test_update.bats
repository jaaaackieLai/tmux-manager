#!/usr/bin/env bats
# tests/test_update.bats - Tests for lib/update.sh

load 'test_helper'

@test "check_remote_version parses VERSION from remote constants" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/update.sh'

        curl() {
            cat <<'EOF'
readonly VERSION=\"2.3.4\"
EOF
        }

        check_remote_version
    "
    [ "$status" -eq 0 ]
    [ "$output" = "2.3.4" ]
}

@test "do_self_update exits early when already on latest version" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/update.sh'

        check_remote_version() { echo \"\$VERSION\"; }
        run_remote_installer() { echo 'INSTALLER_CALLED'; return 0; }

        do_self_update
    "
    [ "$status" -eq 0 ]
    [[ "$output" == *"Already up to date"* ]]
    [[ "$output" != *"INSTALLER_CALLED"* ]]
}

@test "do_self_update runs installer when remote version differs" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/update.sh'

        tmpbin=\$(mktemp -d)
        cat > \"\$tmpbin/tmux-manager\" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
        chmod +x \"\$tmpbin/tmux-manager\"
        PATH=\"\$tmpbin:\$PATH\"

        check_remote_version() { echo '9.9.9'; }
        run_remote_installer() { echo \"INSTALL_DIR:\$1\"; return 0; }

        do_self_update
        status=\$?
        rm -rf \"\$tmpbin\"
        exit \$status
    "
    [ "$status" -eq 0 ]
    [[ "$output" == *"Updating tmux-manager v"*" -> v9.9.9"* ]]
    [[ "$output" == *"INSTALL_DIR:"* ]]
    [[ "$output" == *"Update complete."* ]]
}
