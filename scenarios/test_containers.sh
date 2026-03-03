#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# test_containers.sh
#
# Verifies that all foc-* containers reported in devnet-info.json
# are actually running and that no unexpected foc-* containers
# exist outside the current run.
# ─────────────────────────────────────────────────────────────
set -euo pipefail

SCENARIO_DIR="$(cd "$(dirname "$0")" && pwd)"
source "${SCENARIO_DIR}/lib.sh"
scenario_start "test_containers"

# ── Collect expected container names from devnet-info ────────
EXPECTED=()
EXPECTED+=("$(jq_devnet '.info.lotus.container_name')")
EXPECTED+=("$(jq_devnet '.info.lotus_miner.container_name')")

# Each Curio SP also has a container
SP_COUNT=$(jq_devnet '.info.pdp_sps | length')
for i in $(seq 0 $((SP_COUNT - 1))); do
  EXPECTED+=("$(jq_devnet ".info.pdp_sps[$i].container_name")")
done

# ── Verify each expected container is running ────────────────
for cname in "${EXPECTED[@]}"; do
  STATUS=$(docker inspect -f '{{.State.Status}}' "$cname" 2>/dev/null || echo "missing")
  assert_eq "$STATUS" "running" "container ${cname} is running"
done

# ── Check no unexpected foc-* containers are running ─────────
# All foc-* containers for this devnet run should belong to the expected set.
# Prefer the run-scoped prefix from devnet-info when available, fall back to foc-.
RUN_ID="$(jq_devnet '.info.run_id // ""')"
if [[ -n "$RUN_ID" ]]; then
  NAME_FILTER="foc-${RUN_ID}-"
else
  NAME_FILTER="foc-"
fi
RUNNING=$(docker ps --filter "name=${NAME_FILTER}" --format '{{.Names}}')
for cname in $RUNNING; do
  KNOWN=false
  for exp in "${EXPECTED[@]}"; do
    [[ "$cname" == "$exp" ]] && KNOWN=true && break
  done
  # Portainer is allowed but not in devnet-info
  [[ "$cname" == *"-portainer"* ]] && KNOWN=true
  assert_eq "$KNOWN" "true" "container ${cname} is expected"
done

scenario_end
