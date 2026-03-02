#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# test_containers.sh
#
# Verifies that all foc-* containers reported in devnet-info.json
# are actually running, healthy, and that no zombie foc-*
# containers exist outside the current run.
# ─────────────────────────────────────────────────────────────
set -euo pipefail

SCENARIO_DIR="$(cd "$(dirname "$0")" && pwd)"
source "${SCENARIO_DIR}/lib.sh"
scenario_start "containers"

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

# ── Check no unexpected foc-c-* containers are running ───────
# All foc-c-* containers should belong to the expected set
RUNNING=$(docker ps --filter "name=foc-c-" --format '{{.Names}}')
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
