#!/usr/bin/env bats
# tests/test_ai.bats - Tests for lib/ai.sh

load 'test_helper'

# ─── ai_enabled tests ───────────────────────────────────────────────

@test "ai_enabled returns false when ANTHROPIC_API_KEY is empty" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        unset ANTHROPIC_API_KEY
        ai_enabled
    "
    [ "$status" -ne 0 ]
}

@test "ai_enabled returns false when curl is missing" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        ANTHROPIC_API_KEY='sk-test-key'
        command() {
            if [[ \"\${2:-}\" == 'curl' ]]; then return 1; fi
            builtin command \"\$@\"
        }
        export -f command
        ai_enabled
    "
    [ "$status" -ne 0 ]
}

@test "ai_enabled returns false when jq is missing" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        ANTHROPIC_API_KEY='sk-test-key'
        command() {
            if [[ \"\${2:-}\" == 'jq' ]]; then return 1; fi
            builtin command \"\$@\"
        }
        export -f command
        ai_enabled
    "
    [ "$status" -ne 0 ]
}

@test "ai_enabled returns true when key and tools are present" {
    # Only run if real curl and jq exist
    command -v curl >/dev/null 2>&1 || skip "curl not available"
    command -v jq >/dev/null 2>&1 || skip "jq not available"
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        ANTHROPIC_API_KEY='sk-test-key'
        ai_enabled
    "
    [ "$status" -eq 0 ]
}

# ─── load_ai_results tests ──────────────────────────────────────────
# Use a helper script to run load_ai_results in a fresh shell
# and print results so we can assert on them.

@test "load_ai_results fills AI_SUMMARIES from temp files" {
    local tmpdir="${BATS_TMPDIR}/ai-test-$$"
    mkdir -p "$tmpdir"
    echo "正在開發登入功能" > "${tmpdir}/alpha.summary"
    echo "跑測試中" > "${tmpdir}/beta.summary"

    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        SESSIONS=(alpha beta)
        AI_SUMMARIES=('' '')
        AI_NAMES=('' '')
        AI_TEMP_DIR='${tmpdir}'
        load_ai_results
        echo \"\${AI_SUMMARIES[0]}\"
        echo \"\${AI_SUMMARIES[1]}\"
    "
    [ "$status" -eq 0 ]
    [ "${lines[0]}" = "正在開發登入功能" ]
    [ "${lines[1]}" = "跑測試中" ]

    rm -rf "$tmpdir"
}

@test "load_ai_results fills AI_NAMES from temp files" {
    local tmpdir="${BATS_TMPDIR}/ai-test-$$"
    mkdir -p "$tmpdir"
    echo "login-feature" > "${tmpdir}/alpha.name"
    echo "test-runner" > "${tmpdir}/beta.name"

    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        SESSIONS=(alpha beta)
        AI_SUMMARIES=('' '')
        AI_NAMES=('' '')
        AI_TEMP_DIR='${tmpdir}'
        load_ai_results
        echo \"\${AI_NAMES[0]}\"
        echo \"\${AI_NAMES[1]}\"
    "
    [ "$status" -eq 0 ]
    [ "${lines[0]}" = "login-feature" ]
    [ "${lines[1]}" = "test-runner" ]

    rm -rf "$tmpdir"
}

@test "load_ai_results skips missing files gracefully" {
    local tmpdir="${BATS_TMPDIR}/ai-test-$$"
    mkdir -p "$tmpdir"
    echo "only alpha summary" > "${tmpdir}/alpha.summary"
    # beta files intentionally missing

    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        SESSIONS=(alpha beta)
        AI_SUMMARIES=('' '')
        AI_NAMES=('' '')
        AI_TEMP_DIR='${tmpdir}'
        load_ai_results
        echo \"\${AI_SUMMARIES[0]}\"
        echo \"|\${AI_SUMMARIES[1]}|\"
    "
    [ "$status" -eq 0 ]
    [ "${lines[0]}" = "only alpha summary" ]
    [ "${lines[1]}" = "||" ]

    rm -rf "$tmpdir"
}

@test "load_ai_results does nothing when temp dir missing" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        SESSIONS=(alpha beta)
        AI_SUMMARIES=('keep1' 'keep2')
        AI_TEMP_DIR='/nonexistent/path/$$'
        load_ai_results || true
        echo \"\${AI_SUMMARIES[0]}\"
        echo \"\${AI_SUMMARIES[1]}\"
    "
    [ "$status" -eq 0 ]
    [ "${lines[0]}" = "keep1" ]
    [ "${lines[1]}" = "keep2" ]
}

# ─── load_ai_results return value tests ──────────────────────────────

@test "load_ai_results returns 0 (true) when summaries changed" {
    local tmpdir="${BATS_TMPDIR}/ai-ret-$$"
    mkdir -p "$tmpdir"
    echo "new summary" > "${tmpdir}/alpha.summary"

    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        SESSIONS=(alpha)
        AI_SUMMARIES=('')
        AI_NAMES=('')
        AI_TEMP_DIR='${tmpdir}'
        load_ai_results
    "
    [ "$status" -eq 0 ]

    rm -rf "$tmpdir"
}

@test "load_ai_results returns 1 (false) when summaries unchanged" {
    local tmpdir="${BATS_TMPDIR}/ai-ret-$$"
    mkdir -p "$tmpdir"
    echo "same summary" > "${tmpdir}/alpha.summary"

    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        SESSIONS=(alpha)
        AI_SUMMARIES=('same summary')
        AI_NAMES=('')
        AI_TEMP_DIR='${tmpdir}'
        load_ai_results
    "
    [ "$status" -eq 1 ]

    rm -rf "$tmpdir"
}

@test "load_ai_results returns 1 (false) when temp dir missing" {
    run bash -c "
        source '${LIB_DIR}/constants.sh'
        source '${LIB_DIR}/utils.sh'
        source '${LIB_DIR}/sessions.sh'
        source '${LIB_DIR}/ai.sh'
        SESSIONS=(alpha)
        AI_SUMMARIES=('old')
        AI_TEMP_DIR='/nonexistent/path'
        load_ai_results
    "
    [ "$status" -eq 1 ]
}
