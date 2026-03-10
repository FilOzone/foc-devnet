#!/usr/bin/env bash
# install_precommit_hooks.sh — Optionally installs a pre-commit hook that runs lint.sh
set -euo pipefail

cd "$(cd "$(dirname "$0")/.." && pwd)"

HOOK="$(git rev-parse --git-path hooks)/pre-commit"

mkdir -p "$(dirname "$HOOK")"

if [[ -e "$HOOK" && "${1:-}" != "-f" ]]; then
  echo "Hook already exists: $HOOK (use -f to overwrite)"
  exit 1
fi

cat > "$HOOK" << 'EOF'
#!/usr/bin/env bash
exec "$(git rev-parse --show-toplevel)/scripts/lint.sh"
EOF

chmod +x "$HOOK"
echo "Installed: $HOOK"
