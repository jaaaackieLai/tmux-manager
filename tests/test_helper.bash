#!/usr/bin/env bash
# test_helper.bash - Shared BATS test helper
# Loaded by individual test files via: load 'test_helper'

# Root of the project
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LIB_DIR="${REPO_ROOT}/lib"

# Load a lib module without side effects
# Usage: load_lib constants
load_lib() {
    local name="$1"
    source "${LIB_DIR}/${name}.sh"
}
