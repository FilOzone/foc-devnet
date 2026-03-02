#!/usr/bin/env bash
# Installs the repo's git hooks by pointing core.hooksPath at .githooks/
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
git -C "$REPO_ROOT" config core.hooksPath .githooks
chmod +x "$REPO_ROOT"/.githooks/*
echo "✓ Git hooks installed (.githooks/)"
