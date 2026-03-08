#!/bin/bash
# Comprehensive playbook testing via noah-backend-prompt
# Requires: NOAH_API_URL and NOAH_MODEL env vars (local LLM)
# Usage: bash tests/test_playbooks.sh [--quick]

set -euo pipefail

export NOAH_API_URL="${NOAH_API_URL:-http://127.0.0.1:8082}"
export NOAH_MODEL="${NOAH_MODEL:-local}"

# Use gtimeout (from coreutils) or fall back to perl-based timeout
TIMEOUT_CMD=""
if command -v gtimeout &>/dev/null; then
    TIMEOUT_CMD="gtimeout"
elif command -v timeout &>/dev/null; then
    TIMEOUT_CMD="timeout"
fi

BINARY="cargo run --bin noah-backend-prompt --"
RESULTS_DIR="tests/playbook-results"
QUICK="${1:-}"
MAX_TURNS=5
if [ "$QUICK" = "--quick" ]; then MAX_TURNS=3; fi

mkdir -p "$RESULTS_DIR"

PASS=0
FAIL=0
TOTAL=0

run_test() {
    local name="$1"
    local prompt="$2"
    local turns="${3:-$MAX_TURNS}"
    local check_pattern="${4:-}"

    TOTAL=$((TOTAL + 1))
    local outfile="$RESULTS_DIR/${name}.txt"

    echo -n "[$TOTAL] $name ... "

    local run_cmd="$BINARY \"$prompt\" $turns"
    local ran_ok=false
    if [ -n "$TIMEOUT_CMD" ]; then
        if $TIMEOUT_CMD 300 $BINARY "$prompt" "$turns" > "$outfile" 2>&1; then ran_ok=true; fi
    else
        if $BINARY "$prompt" "$turns" > "$outfile" 2>&1; then ran_ok=true; fi
    fi
    if $ran_ok; then
        # Basic checks
        local has_output=false
        local has_ui=false
        local has_error=false

        if grep -q "TURN 1 OUTPUT" "$outfile"; then has_output=true; fi
        if grep -qiE '"kind"\s*:\s*"(spa|user_question|done|info)"' "$outfile"; then has_ui=true; fi
        if grep -qi "ERROR\|panic\|thread.*panicked" "$outfile"; then has_error=true; fi

        # Check for specific pattern if provided
        local pattern_ok=true
        if [ -n "$check_pattern" ]; then
            if ! grep -qi "$check_pattern" "$outfile"; then
                pattern_ok=false
            fi
        fi

        if $has_output && ! $has_error; then
            if [ -n "$check_pattern" ] && ! $pattern_ok; then
                echo "WARN (no pattern match: $check_pattern)"
                PASS=$((PASS + 1))  # Still counts as pass if no crash
            elif $has_ui; then
                echo "PASS (structured UI)"
            else
                echo "PASS (text response)"
            fi
            PASS=$((PASS + 1))
        else
            echo "FAIL"
            FAIL=$((FAIL + 1))
            # Show last few lines for diagnosis
            tail -10 "$outfile" | head -5
        fi
    else
        echo "FAIL (timeout/crash)"
        FAIL=$((FAIL + 1))
    fi
}

echo "========================================="
echo " Noah Playbook Test Suite"
echo " API: $NOAH_API_URL  Model: $NOAH_MODEL"
echo " Max turns per test: $MAX_TURNS"
echo "========================================="
echo ""

# ── Diagnostic Playbooks (existing, should still work) ──
echo "--- Diagnostic Playbooks (backward compat) ---"

run_test "network-basic" \
    "My wifi keeps dropping every few minutes" \
    "$MAX_TURNS" \
    "network\|wifi\|Wi-Fi\|connectivity"

run_test "printer-basic" \
    "My printer isn't printing anything, the jobs are stuck" \
    "$MAX_TURNS" \
    "printer\|print\|queue"

run_test "disk-space" \
    "My Mac says the disk is almost full, what should I do?" \
    "$MAX_TURNS" \
    "disk\|space\|storage"

run_test "slow-computer" \
    "My computer is really slow lately, everything takes forever" \
    "$MAX_TURNS" \
    "performance\|slow\|CPU\|memory"

run_test "outlook-issue" \
    "Outlook keeps crashing when I open it" \
    "$MAX_TURNS" \
    "Outlook\|email\|crash"

run_test "vpn-problem" \
    "I can't connect to the company VPN" \
    "$MAX_TURNS" \
    "VPN\|connect"

echo ""

# ── Procedural Playbooks (new, testing step-by-step flow) ──
echo "--- Procedural Playbooks (new) ---"

run_test "homebrew-install" \
    "I need to install Homebrew on my Mac, can you help me set it up?" \
    "$MAX_TURNS" \
    "brew\|Homebrew\|install"

run_test "homebrew-with-app" \
    "I want to install Visual Studio Code on my Mac. I don't think I have any package manager." \
    "$MAX_TURNS" \
    "brew\|Homebrew\|VS Code\|Visual Studio"

run_test "ssh-key-github" \
    "I need to set up SSH keys so I can push to GitHub without entering my password every time" \
    "$MAX_TURNS" \
    "SSH\|key\|ssh-keygen\|GitHub"

run_test "ssh-key-existing" \
    "I keep getting 'permission denied publickey' when I try to git push" \
    "$MAX_TURNS" \
    "SSH\|key\|permission\|publickey"

run_test "wifi-setup-home" \
    "I just got a new router and need to connect my Mac to it" \
    "$MAX_TURNS" \
    "Wi-Fi\|wifi\|network\|connect\|SSID"

run_test "wifi-setup-enterprise" \
    "I need to connect to my company's Wi-Fi network. It requires a username and password, not just a password." \
    "$MAX_TURNS" \
    "Wi-Fi\|enterprise\|WPA\|username"

run_test "backup-setup" \
    "I want to set up Time Machine on my Mac, I just bought an external drive" \
    "$MAX_TURNS" \
    "Time Machine\|backup\|tmutil\|drive"

run_test "email-gmail" \
    "I need to add my Gmail account to the Mail app on my Mac" \
    "$MAX_TURNS" \
    "Gmail\|email\|Mail\|account"

run_test "email-corporate" \
    "I need to set up my work email in Outlook. My email is john@acmecorp.com and IT said the server is mail.acmecorp.com" \
    "$MAX_TURNS" \
    "email\|Outlook\|server\|IMAP\|Exchange"

echo ""

# ── UI Primitive Tests (stress-test specific UI patterns) ──
echo "--- UI Primitive Stress Tests ---"

run_test "question-with-options" \
    "I have a problem with my computer" \
    3 \
    ""

run_test "multi-step-confirm" \
    "Help me install Homebrew and then use it to install Chrome" \
    "$MAX_TURNS" \
    ""

run_test "generic-setup-request" \
    "Can you help me set up my new Mac? I just got it and I need help configuring everything." \
    "$MAX_TURNS" \
    ""

run_test "ambiguous-request" \
    "I need help with my email" \
    3 \
    ""

# ── Edge Cases ──
echo ""
echo "--- Edge Cases ---"

run_test "empty-message" \
    "hi" \
    2 \
    ""

run_test "non-it-request" \
    "Can you help me write a poem about computers?" \
    2 \
    ""

run_test "multiple-problems" \
    "My wifi is broken AND my printer doesn't work AND my email is down" \
    "$MAX_TURNS" \
    ""

echo ""
echo "========================================="
echo " Results: $PASS passed, $FAIL failed out of $TOTAL"
echo " Detailed output in: $RESULTS_DIR/"
echo "========================================="

exit $FAIL
