#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# setup-scenarios-prerequisites.sh — Install all scenario test
# dependencies so that scenario scripts only run tests, not setup.
#
# Installs (if not already present):
#   1. Foundry (cast, forge)
#   2. Python 3.11.10 via pyenv (for cqlsh / Cassandra)
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
FOUNDRY_VERSION="v1.6.0-rc1"
PYENV_VERSION="v2.5.3"
CASSANDRA_VERSION="5.0.7"
PYTHON_VERSION="3.11.10"
PYENV_ROOT="${PYENV_ROOT:-$HOME/.pyenv}"
PYTHON_BIN="${PYENV_ROOT}/versions/${PYTHON_VERSION}/bin/python3"
CASSANDRA_URL="https://archive.apache.org/dist/cassandra/${CASSANDRA_VERSION}/apache-cassandra-${CASSANDRA_VERSION}-bin.tar.gz"
CASSANDRA_DIR="$HOME/.foc-devnet/artifacts/cassandra"
FOUNDRY_DIR="$HOME/.foc-devnet/artifacts/foundry/bin"
CASSANDRA_HOME="${CASSANDRA_DIR}/apache-cassandra-${CASSANDRA_VERSION}"

verify_checksum() {
  local file="$1" expected="$2"
  local actual
  actual=$(sha256sum "$file" | awk '{print $1}')
  if [[ "$actual" != "$expected" ]]; then
    fail "Checksum mismatch for $file (got $actual, want $expected)"
  fi
}

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

CAST="$FOUNDRY_DIR/cast"

if [[ -x "$CAST" ]]; then
  pass "Foundry already installed (cast @ $CAST)"
else
  info "Installing Foundry ${FOUNDRY_VERSION} to $FOUNDRY_DIR..."
  mkdir -p "$FOUNDRY_DIR"
  ARCH="$(uname -m)"
  case "$ARCH" in
    x86_64)  ARCH="amd64" ;;
    aarch64|arm64) ARCH="arm64" ;;
    *) fail "Unsupported architecture: $ARCH" ;;
  esac
  OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
  TARBALL_NAME="foundry_${FOUNDRY_VERSION}_${OS}_${ARCH}.tar.gz"
  TARBALL_URL="https://github.com/foundry-rs/foundry/releases/download/${FOUNDRY_VERSION}/${TARBALL_NAME}"
  TARBALL_PATH="/tmp/${TARBALL_NAME}"
  info "Downloading $TARBALL_URL"
  curl -fsSL -o "$TARBALL_PATH" "$TARBALL_URL"
  tar -xzf "$TARBALL_PATH" -C "$FOUNDRY_DIR"
  rm -f "$TARBALL_PATH"
  if [[ -x "$CAST" ]]; then
    pass "Foundry installed successfully"
  else
    fail "Foundry installation failed — cast not found at $CAST"
  fi
fi

# ── 2. Python 3.11.10 via pyenv (for cqlsh) ─────────────────
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
      info "Installing pyenv ${PYENV_VERSION} from GitHub tarball..."
      PYENV_TARBALL="/tmp/pyenv-${PYENV_VERSION}.tar.gz"
      curl -fsSL -o "$PYENV_TARBALL" \
        "https://github.com/pyenv/pyenv/archive/refs/tags/${PYENV_VERSION}.tar.gz"
      mkdir -p "${PYENV_ROOT}"
      tar -xzf "$PYENV_TARBALL" -C "${PYENV_ROOT}" --strip-components=1
      rm -f "$PYENV_TARBALL"
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
