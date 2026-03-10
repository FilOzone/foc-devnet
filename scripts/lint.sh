#!/usr/bin/env bash
# lint.sh — FIX=1 (default) auto-fixes; FIX=0 check-only (CI)
set -euo pipefail

FIX="${FIX:-1}"
FAIL=0

pass() { printf "\033[32m✓\033[0m %s\n" "$1"; }
fail() { printf "\033[31m✗\033[0m %s\n" "$1"; FAIL=1; }
skip() { printf "\033[33m⊘\033[0m %s (tool not found)\n" "$1"; }

cd "$(cd "$(dirname "$0")/.." && pwd)"

if command -v cargo &>/dev/null; then
  if [[ "$FIX" == "1" ]]; then
    cargo fmt --all && pass "cargo fmt" || fail "cargo fmt"
  else
    cargo fmt --all -- --check &>/dev/null && pass "cargo fmt" || fail "cargo fmt (run lint.sh to fix)"
  fi
  cargo clippy --all-targets --all-features -- -D warnings &>/dev/null && pass "cargo clippy" || fail "cargo clippy"
else
  skip "cargo"
fi

if find scenarios -name '*.py' 2>/dev/null | grep -q .; then
  if command -v black &>/dev/null; then
    if [[ "$FIX" == "1" ]]; then
      black scenarios/ &>/dev/null && pass "black" || fail "black"
    else
      black --check scenarios/ &>/dev/null && pass "black" || fail "black (run lint.sh to fix)"
    fi
  else
    skip "black (pip install black)"
  fi
fi

[[ $FAIL -eq 0 ]] && printf "\033[32m✓ All checks passed.\033[0m\n" || { printf "\033[31m✗ Linting failed.\033[0m\n"; exit 1; }
