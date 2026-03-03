#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────
# test_basic_balances.sh
#
# Installs Foundry (cast), then verifies every user account on
# the devnet has a positive tFIL balance and a positive USDFC
# token balance.
# ─────────────────────────────────────────────────────────────
set -euo pipefail

SCENARIO_DIR="$(cd "$(dirname "$0")" && pwd)"
source "${SCENARIO_DIR}/lib.sh"
scenario_start "test_basic_balances"

# ── Ensure Foundry is available ──────────────────────────────
if ! command -v cast &>/dev/null; then
  info "Installing Foundry …"
  export SHELL=/bin/bash
  curl -sSL https://foundry.paradigm.xyz | bash
  export PATH="$HOME/.foundry/bin:$PATH"
  "$HOME/.foundry/bin/foundryup"
fi
assert_ok command -v cast "cast is installed"

# ── Read devnet info ─────────────────────────────────────────
RPC_URL=$(jq_devnet '.info.lotus.host_rpc_url')
USDFC_ADDR=$(jq_devnet '.info.contracts.mockusdfc_addr')
USER_COUNT=$(jq_devnet '.info.users | length')
assert_gt "$USER_COUNT" 0 "at least one user exists"

# ── Check each user ──────────────────────────────────────────
for i in $(seq 0 $((USER_COUNT - 1))); do
  NAME=$(jq_devnet ".info.users[$i].name")
  ADDR=$(jq_devnet ".info.users[$i].evm_addr")

  # Native FIL balance (returned in wei)
  FIL_WEI=$(cast balance "$ADDR" --rpc-url "$RPC_URL")
  assert_gt "$FIL_WEI" 0 "${NAME} FIL balance > 0"

  # MockUSDFC ERC-20 balance
  USDFC_WEI=$(cast call "$USDFC_ADDR" "balanceOf(address)(uint256)" "$ADDR" --rpc-url "$RPC_URL")
  assert_gt "$USDFC_WEI" 0 "${NAME} USDFC balance > 0"
done

scenario_end
