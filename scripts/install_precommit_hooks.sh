#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# install_precommit_hooks.sh — Install pre-commit hooks
#
# This script installs a pre-commit hook that runs lint.sh
# in check mode (FIX=0) before each commit.
#
# Usage:
#   ./scripts/install_precommit_hooks.sh
# ─────────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

# Get the actual git hooks directory (works for both regular repos and worktrees)
GIT_HOOKS_DIR="$(git rev-parse --git-path hooks)"
PRE_COMMIT_HOOK="$GIT_HOOKS_DIR/pre-commit"

# Ensure hooks directory exists
mkdir -p "$GIT_HOOKS_DIR"

# Create the pre-commit hook
cat > "$PRE_COMMIT_HOOK" << 'EOF'
#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# Pre-commit hook for foc-devnet
#
# Runs lint.sh in check mode before each commit.
# Skip with: git commit --no-verify
# Auto-fix mode: FIX=1 git commit
# ─────────────────────────────────────────────────────────────
set -euo pipefail

FIX="${FIX:-0}"

REPO_ROOT="$(git rev-parse --show-toplevel)"

echo "Running pre-commit linting..."
echo ""

if FIX="$FIX" "$REPO_ROOT/scripts/lint.sh"; then
    exit 0
else
    echo ""
    echo "Pre-commit hook failed. To skip this hook, use: git commit --no-verify"
    echo "To auto-fix issues, use: FIX=1 git commit"
    exit 1
fi
EOF

# Make the hook executable
chmod +x "$PRE_COMMIT_HOOK"
