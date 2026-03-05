#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# lint.sh — Unified linting script for foc-devnet
#
# Runs linters and formatters for Rust and Python code.
# Designed to work both locally and in CI.
#
# Modes:
#   FIX=1 (default) — Auto-fix issues where possible
#   FIX=0           — Check only, fail on issues
#
# Usage:
#   ./scripts/lint.sh          # Fix mode
#   FIX=0 ./scripts/lint.sh    # Check mode (CI)
#
# Requirements:
#   Rust: cargo, rustfmt, clippy
#   Python: black, ruff (or pip install black ruff)
# ─────────────────────────────────────────────────────────────
set -euo pipefail

FIX="${FIX:-1}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m'

FAIL=0

pass() { printf "${GREEN}✓${NC} %s\n" "$1"; }
fail() { printf "${RED}✗${NC} %s\n" "$1"; FAIL=1; }
skip() { printf "${YELLOW}⊘${NC} %s (skipped — tool not found)\n" "$1"; }
fixed() { printf "${BLUE}⟳${NC} %s (auto-fixed)\n" "$1"; }
info() { printf "${BLUE}ℹ${NC} %s\n" "$1"; }

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

info "Checking Rust code..."

if command -v cargo &>/dev/null; then
  # ── cargo fmt ──
  if [[ "$FIX" == "1" ]]; then
    if cargo fmt --all; then
      fixed "cargo fmt"
    else
      fail "cargo fmt failed"
    fi
  else
    if cargo fmt --all -- --check &>/dev/null; then
      pass "cargo fmt"
    else
      fail "cargo fmt — run './scripts/lint.sh' or 'cargo fmt --all' to fix"
    fi
  fi

  # ── cargo clippy ──
  if cargo clippy --all-targets --all-features -- -D warnings &>/dev/null; then
    pass "cargo clippy"
  else
    fail "cargo clippy — fix warnings before committing"
  fi
else
  skip "cargo (Rust checks)"
fi

echo ""

info "Checking Python code in scenarios/..."

PYTHON_FILES=$(find scenarios -name '*.py' 2>/dev/null || true)

if [[ -z "$PYTHON_FILES" ]]; then
  skip "Python files (none found in scenarios/)"
else
  # ── black (formatter) ──
  if command -v black &>/dev/null; then
    if [[ "$FIX" == "1" ]]; then
      if black scenarios/ &>/dev/null; then
        fixed "black (Python formatter)"
      else
        fail "black failed"
      fi
    else
      if black --check scenarios/ &>/dev/null; then
        pass "black (Python formatter)"
      else
        fail "black — run './scripts/lint.sh' or 'black scenarios/' to fix"
      fi
    fi
  else
    skip "black (install with: pip install black)"
  fi
fi

echo ""

echo "════════════════════════════════════════════════════════"
if [[ $FAIL -ne 0 ]]; then
  printf "${RED}✗ Linting failed.${NC}\n"
  if [[ "$FIX" == "0" ]]; then
    echo "  Run './scripts/lint.sh' (FIX=1 mode) to auto-fix issues."
  fi
  exit 1
else
  printf "${GREEN}✓ All linting checks passed.${NC}\n"
fi
echo "════════════════════════════════════════════════════════"
