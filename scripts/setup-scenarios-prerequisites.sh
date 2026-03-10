#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# setup-scenarios-prerequisites.sh — Install all scenario test
# dependencies so that scenario scripts only run tests, not setup.
#
# Installs (if not already present):
#   1. Foundry (cast, forge)
#   2. Python 3.11.15 via pyenv (for cqlsh / Cassandra)
#   3. cqlsh via Apache Cassandra tarball
#
# Also verifies that git, node, and pnpm are available.
#
# Usage:
#   ./scripts/setup-scenarios-prerequisites.sh
# ─────────────────────────────────────────────────────────────
set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

pass()  { printf "${GREEN}✓${NC} %s\n" "$1"; }
fail()  { printf "${RED}✗${NC} %s\n" "$1"; exit 1; }
info()  { printf "${BLUE}ℹ${NC} %s\n" "$1"; }

# ── Constants ────────────────────────────────────────────────
CASSANDRA_VERSION="5.0.6"
PYTHON_VERSION="3.11.15"
PYENV_ROOT="${PYENV_ROOT:-$HOME/.pyenv}"
PYTHON_BIN="${PYENV_ROOT}/versions/${PYTHON_VERSION}/bin/python3"
CASSANDRA_URL="https://dlcdn.apache.org/cassandra/${CASSANDRA_VERSION}/apache-cassandra-${CASSANDRA_VERSION}-bin.tar.gz"
CASSANDRA_DIR="$HOME/.foc-devnet/artifacts/cassandra"
CASSANDRA_HOME="${CASSANDRA_DIR}/apache-cassandra-${CASSANDRA_VERSION}"

# ── 0. Verify basic system tools ────────────────────────────
info "Checking basic system tools..."

for tool in git node pnpm; do
  if command -v "$tool" &>/dev/null; then
    pass "$tool is installed ($(command -v "$tool"))"
  else
    fail "$tool is required but not found. Please install it first."
  fi
done

# ── 1. Foundry (cast / forge) ───────────────────────────────
info "Checking Foundry..."

if command -v cast &>/dev/null; then
  pass "Foundry already installed (cast @ $(command -v cast))"
else
  info "Installing Foundry..."
  curl -sSL https://foundry.paradigm.xyz | bash
  export PATH="$HOME/.foundry/bin:$PATH"
  "$HOME/.foundry/bin/foundryup"
  if command -v cast &>/dev/null; then
    pass "Foundry installed successfully"
  else
    fail "Foundry installation failed — cast not found on PATH"
  fi
fi

# ── 2. Python 3.11.15 via pyenv (for cqlsh) ─────────────────
info "Checking Python ${PYTHON_VERSION} via pyenv..."

CUSTOM_PYTHON="${PYTHON_BIN}"

if [[ -x "$CUSTOM_PYTHON" ]]; then
  pass "Python ${PYTHON_VERSION} already installed (${CUSTOM_PYTHON})"
else
  # Install pyenv if not present
  if ! command -v pyenv &>/dev/null; then
    if [[ -x "${PYENV_ROOT}/bin/pyenv" ]]; then
      export PATH="${PYENV_ROOT}/bin:$PATH"
    else
      info "Installing pyenv..."
      curl -fsSL https://pyenv.run | bash
      export PATH="${PYENV_ROOT}/bin:$PATH"
    fi
  fi
  pass "pyenv available ($(command -v pyenv))"

  info "Installing Python ${PYTHON_VERSION} via pyenv..."
  pyenv install -s "${PYTHON_VERSION}"
  if [[ -x "$CUSTOM_PYTHON" ]]; then
    pass "Python ${PYTHON_VERSION} installed (${CUSTOM_PYTHON})"
  else
    fail "Python ${PYTHON_VERSION} installation failed — binary not found at ${CUSTOM_PYTHON}"
  fi
fi

# ── 3. cqlsh (Apache Cassandra tarball) ──────────────────────
info "Checking cqlsh..."

CQLSH="${CASSANDRA_HOME}/bin/cqlsh"

if [[ -x "$CQLSH" ]]; then
  CQLSH_VERSION="$(CQLSH_PYTHON="$CUSTOM_PYTHON" "$CQLSH" --version 2>&1 || true)"
  pass "cqlsh already installed (${CQLSH_VERSION})"
else
  info "Downloading Apache Cassandra ${CASSANDRA_VERSION} for cqlsh..."
  mkdir -p "$CASSANDRA_DIR"
  TARBALL="${CASSANDRA_DIR}/apache-cassandra-${CASSANDRA_VERSION}-bin.tar.gz"
  curl -fL -o "$TARBALL" "$CASSANDRA_URL"
  tar -xzf "$TARBALL" -C "$CASSANDRA_DIR"
  if [[ -x "$CQLSH" ]]; then
    CQLSH_VERSION="$(CQLSH_PYTHON="$CUSTOM_PYTHON" "$CQLSH" --version 2>&1 || true)"
    pass "cqlsh installed (${CQLSH_VERSION})"
  else
    fail "cqlsh installation failed — ${CQLSH} not found after extraction"
  fi
fi

# ── Done ─────────────────────────────────────────────────────
echo ""
echo "════════════════════════════════════════════════════════"
printf "${GREEN}✓ All scenario prerequisites are installed.${NC}\n"
echo "════════════════════════════════════════════════════════"
