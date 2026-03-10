#!/usr/bin/env bash
# install_precommit_hooks.sh — Optionally installs a pre-commit hook that runs lint.sh
set -euo pipefail

cd "$(cd "$(dirname "$0")/.." && pwd)"

HOOK="$(git rev-parse --git-path hooks)/pre-commit"
mkdir -p "$(dirname "$HOOK")"
printf '#!/usr/bin/env bash\nexec "$(git rev-parse --show-toplevel)/scripts/lint.sh"\n' > "$HOOK"
chmod +x "$HOOK"
echo "Installed: $HOOK"
