#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# run.sh — Scenario test runner.
#
# Executes every scenario listed in order.sh against the
# currently-running devnet, collects results, prints a report,
# and (when REPORTING=true) files a GitHub issue.
#
# ── Running locally ──────────────────────────────────────────
#
#   Prerequisites:
#     - A running foc-devnet cluster  (./foc-devnet start)
#     - jq, python3, docker on PATH
#     - (optional) Foundry — installed automatically by
#       test_basic_balances if missing
#
#   Quick start:
#     ./foc-devnet start          # bring up the devnet
#     bash scenarios/run.sh        # run all scenarios
#     cat ~/.foc-devnet/state/latest/scenario_*.md  # read the report
#
#   Run a single scenario:
#     bash scenarios/test_containers.sh
#
#   Override devnet-info path (e.g. an older run):
#     DEVNET_INFO=~/.foc-devnet/state/<run-id>/devnet-info.json \
#       bash scenarios/run.sh
#
#   File a GitHub issue on failure (needs `gh` CLI + auth):
#     REPORTING=true bash scenarios/run.sh
#
#   Always file an issue, even on success:
#     REPORTING=true SKIP_REPORT_ON_PASS=false bash scenarios/run.sh
#
# ── Environment variables ────────────────────────────────────
#   DEVNET_INFO          — path to devnet-info.json (auto-detected)
#   REPORTING            — "true" to create a GitHub issue
#   SKIP_REPORT_ON_PASS  — "true" (default) skips the issue when
#                          all scenarios pass
#   GITHUB_SERVER_URL, GITHUB_REPOSITORY, GITHUB_RUN_ID
#                        — set automatically by GitHub Actions
# ─────────────────────────────────────────────────────────────
set -euo pipefail

SCENARIO_DIR="$(cd "$(dirname "$0")" && pwd)"
REPORT_DIR="${REPORT_DIR:-$HOME/.foc-devnet/state/latest}"
REPORTING="${REPORTING:-false}"
SKIP_REPORT_ON_PASS="${SKIP_REPORT_ON_PASS:-true}"

# ── Bootstrap ────────────────────────────────────────────────
# Ensure report directory exists before cleaning/writing artifacts
mkdir -p "${REPORT_DIR}"
# Clean previous scenario artifacts (but not the whole state dir)
rm -f "${REPORT_DIR}"/scenario_*.md "${REPORT_DIR}/results.csv"
source "${SCENARIO_DIR}/order.sh"

TOTAL=0
PASSED=0
FAILED=0
FAILED_NAMES=()
START_TS=$(date +%s)

# ── Execute scenarios ────────────────────────────────────────
for name in "${SCENARIOS[@]}"; do
  script="${SCENARIO_DIR}/${name}.sh"
  if [[ ! -f "$script" ]]; then
    echo "[SKIP] ${name}.sh not found"
    continue
  fi

  ((TOTAL++)) || true
  echo ""
  # Each scenario runs in a subshell so a failure doesn't kill the runner
  if bash "$script"; then
    ((PASSED++)) || true
  else
    ((FAILED++)) || true
    FAILED_NAMES+=("$name")
  fi
done

ELAPSED=$(($(date +%s) - START_TS))

# ── Build report ─────────────────────────────────────────────
REPORT="${REPORT_DIR}/scenario_$(date -u +%Y%m%d_%H%M%S).md"
{
  echo "# Scenario Test Report"
  echo ""
  echo "| Metric | Value |"
  echo "|--------|-------|"
  echo "| Total  | ${TOTAL} |"
  echo "| Passed | ${PASSED} |"
  echo "| Failed | ${FAILED} |"
  echo "| Duration | ${ELAPSED}s |"
  echo ""

  # Per-scenario detail from the CSV each scenario appended
  if [[ -f "${REPORT_DIR}/results.csv" ]]; then
    echo "## Details"
    echo ""
    echo "| Status | Scenario | Passed | Failed |"
    echo "|--------|----------|--------|--------|"
    while IFS='|' read -r st sc pa fa; do
      icon="✅"
      [[ "$st" == "FAIL" ]] && icon="❌"
      echo "| ${icon} ${st} | ${sc} | ${pa} | ${fa} |"
    done <"${REPORT_DIR}/results.csv"
  fi

  if [[ ${#FAILED_NAMES[@]} -gt 0 ]]; then
    echo ""
    echo "## Failed scenarios"
    echo ""
    for n in "${FAILED_NAMES[@]}"; do echo "- \`${n}\`"; done
  fi
} >"$REPORT"

# Print to stdout as well
cat "$REPORT"

# ── GitHub issue (only when REPORTING=true) ──────────────────
SHOULD_REPORT=false
if [[ "$REPORTING" == "true" ]]; then
  if [[ $FAILED -gt 0 ]]; then
    SHOULD_REPORT=true
  elif [[ "$SKIP_REPORT_ON_PASS" != "true" ]]; then
    SHOULD_REPORT=true
  fi
fi

if [[ "$SHOULD_REPORT" == "true" ]]; then
  RUN_URL="${GITHUB_SERVER_URL:-https://github.com}/${GITHUB_REPOSITORY:-unknown}/actions/runs/${GITHUB_RUN_ID:-0}"
  STATUS_EMOJI="✅"
  [[ $FAILED -gt 0 ]] && STATUS_EMOJI="❌"
  ISSUE_TITLE="${STATUS_EMOJI} Scenario report: ${PASSED}/${TOTAL} passed ($(date -u +%Y-%m-%d))"
  ISSUE_BODY="$(cat "$REPORT")

---
[View workflow run](${RUN_URL})"

  LABELS="scenario-report"
  [[ $FAILED -gt 0 ]] && LABELS="scenario-report,bug"
  gh issue create \
    --title "$ISSUE_TITLE" \
    --body "$ISSUE_BODY" \
    --label "$LABELS" \
    || echo "[WARN] Could not create GitHub issue (gh CLI missing or auth failed)"
fi

# ── Exit code reflects overall result ────────────────────────
[[ $FAILED -eq 0 ]]
