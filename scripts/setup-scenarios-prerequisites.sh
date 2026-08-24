#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# setup-scenarios-prerequisites.sh — Install all scenario test
# dependencies so that scenario scripts only run tests, not setup.
#
# Installs Foundry (cast, forge) if not already present, and verifies
# that git, Node.js 24+, npm, and pnpm are available.
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
FOUNDRY_DIR="$HOME/.foc-devnet/artifacts/foundry/bin"

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

for tool in git node npm pnpm; do
  if command -v "$tool" &>/dev/null; then
    pass "$tool is installed ($(command -v "$tool"))"
  else
    fail "$tool is required but not found. Please install it first."
  fi
done

NODE_MAJOR=$(node -p 'process.versions.node.split(".")[0]')
if (( NODE_MAJOR < 24 )); then
  fail "Node.js 24 or newer is required for native TypeScript scenarios."
fi

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

# ── Done ─────────────────────────────────────────────────────
echo ""
echo "════════════════════════════════════════════════════════"
printf "${GREEN}✓ All scenario prerequisites are installed.${NC}\n"
echo "════════════════════════════════════════════════════════"
