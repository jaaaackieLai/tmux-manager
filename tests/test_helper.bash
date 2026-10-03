#!/usr/bin/env bash
# test_helper.bash - Shared BATS test helper
# Loaded by individual test files via: load 'test_helper'

# Root of the project
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# 可攜的逾時：macOS 沒有 GNU timeout；perl 兩邊都有，SIGALRM 預設會結束程序。
with_timeout() {
    local seconds="$1"
    shift
    perl -e 'alarm shift; exec @ARGV' "$seconds" "$@"
}
