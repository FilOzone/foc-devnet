#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# lib.sh — Shared helpers for scenario tests.
#
# Sourced (not executed) by each test_*.sh script.
# Provides: assertions, devnet-info access, and result tracking.
#
# ── Writing a new scenario ───────────────────────────────────
#
#   1. Create scenario/test_<name>.sh with this skeleton:
#
#        #!/usr/bin/env bash
#        set -euo pipefail
#        SCENARIO_DIR="$(cd "$(dirname "$0")" && pwd)"
#        source "${SCENARIO_DIR}/lib.sh"
#        scenario_start "<name>"
#
#        # ... your checks using assert_*, jq_devnet, etc. ...
#
#        scenario_end
#
#   2. Add "test_<name>" to the SCENARIOS array in order.sh.
#   3. chmod +x scenario/test_<name>.sh
#   4. Run:  bash scenario/test_<name>.sh
#
# ── Available helpers ────────────────────────────────────────
#   jq_devnet <filter>           — query devnet-info.json
#   assert_eq  <a> <b> <msg>     — equality check
#   assert_gt  <a> <b> <msg>     — integer greater-than
#   assert_not_empty <v> <msg>   — value is non-empty
#   assert_ok  <cmd...> <msg>    — command exits 0
#   info / ok / fail             — logging
# ─────────────────────────────────────────────────────────────
# shellcheck disable=SC2034  # Variables here are used by scripts that source this file
set -euo pipefail

# ── Paths ────────────────────────────────────────────────────
DEVNET_INFO="${DEVNET_INFO:-$HOME/.foc-devnet/state/latest/devnet-info.json}"
SCENARIO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPORT_DIR="${REPORT_DIR:-$HOME/.foc-devnet/state/latest}"

# Per-scenario counters (reset by scenario_start)
_PASS=0
_FAIL=0
_SCENARIO_NAME=""

# ── devnet-info helpers ──────────────────────────────────────

# Shorthand: jq_devnet '.info.users[0].evm_addr'
jq_devnet() { jq -r "$1" "$DEVNET_INFO"; }

# ── Logging ──────────────────────────────────────────────────
_log() { printf "[%s] %s\n" "$1" "$2"; }
info() { _log "[INFO]" "$*"; }
ok() {
  _log "[ OK ]" "$*"
  ((_PASS++)) || true
}
fail() {
  _log "[FAIL]" "$*"
  ((_FAIL++)) || true
}

# ── Assertions ───────────────────────────────────────────────

# assert_eq <actual> <expected> <message>
assert_eq() {
  if [[ "$1" == "$2" ]]; then ok "$3"; else fail "$3 (got '$1', want '$2')"; fi
}

# assert_not_empty <value> <message>
assert_not_empty() {
  if [[ -n "$1" ]]; then ok "$2"; else fail "$2 (empty)"; fi
}

# assert_gt <actual_number> <threshold> <message>
# Both arguments are treated as integers (wei-scale is fine).
assert_gt() {
  if python3 -c "import sys; sys.exit(0 if int('$1') > int('$2') else 1)" 2>/dev/null; then
    ok "$3"
  else
    fail "$3 (got '$1', want > '$2')"
  fi
}

# assert_ok <command ...> <message (last arg)>
# Runs the command; passes if exit-code == 0.
assert_ok() {
  local msg="${*: -1}"
  local cmd=("${@:1:$#-1}")
  if "${cmd[@]}" >/dev/null 2>&1; then ok "$msg"; else fail "$msg"; fi
}

# ── Scenario lifecycle ───────────────────────────────────────

scenario_start() {
  _SCENARIO_NAME="$1"
  _PASS=0
  _FAIL=0
  info "━━━ START: ${_SCENARIO_NAME} ━━━"
}

scenario_end() {
  local total=$((_PASS + _FAIL))
  local status="PASS"
  [[ $_FAIL -gt 0 ]] && status="FAIL"
  info "━━━ END: ${_SCENARIO_NAME}  ${_PASS}/${total} passed  [${status}] ━━━"
  # Write machine-readable result line for the runner
  mkdir -p "$REPORT_DIR"
  echo "${status}|${_SCENARIO_NAME}|${_PASS}|${_FAIL}" >>"${REPORT_DIR}/results.csv"
  [[ $_FAIL -eq 0 ]]
}
