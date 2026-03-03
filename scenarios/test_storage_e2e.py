#!/usr/bin/env python3
# Inspired by https://raw.githubusercontent.com/FilOzone/synapse-sdk/refs/heads/master/utils/example-storage-e2e.js
#
# Verifies the devnet is ready for end-to-end FOC warm storage interactions:
#
#   1. At least one PDP service provider exists (mirrors "provider selection")
#   2. Every SP is approved in the FWSS contract   (mirrors allowanceCheck)
#   3. Every SP is endorsed in the Endorsements contract
#   4. Every SP's PDP service URL is reachable
#   5. USER_1 has a USDFC balance                  (mirrors "Checking Balances")
#   6. USER_1 has deposited USDFC into FilecoinPay  (mirrors "Preflight Upload Check")
#   7. FWSS is set as an operator for USER_1        (mirrors allowanceCheck operator step)
from scenarios_py.run import *
import urllib.request

# ── ABI fragments used with `cast call` ──────────────────────────────────────
_SIG_PAYMENT_BALANCE   = "balance(address)(uint256)"
_SIG_IS_OPERATOR       = "isOperator(address,address)(bool)"
_SIG_SP_APPROVED       = "isProviderApproved(uint256)(bool)"
_SIG_IS_ENDORSED       = "isEndorsed(address)(bool)"


def _cast(contract: str, sig: str, *args, rpc: str) -> str:
    """Call a read-only contract function via cast and return the raw output."""
    joined = " ".join(args)
    return sh(f"cast call {contract} '{sig}' {joined} --rpc-url {rpc}")


def _check_sps(sps: list, contracts: dict, rpc: str) -> None:
    """Assert each SP is approved in FWSS and endorsed in the Endorsements contract."""
    fwss    = contracts["fwss_service_proxy_addr"]
    endorsements = contracts["endorsements_addr"]

    assert_gt(len(sps), 0, "at least one PDP service provider exists")

    for sp in sps:
        pid  = sp["provider_id"]
        addr = sp["eth_addr"]

        approved = _cast(fwss, _SIG_SP_APPROVED, str(pid), rpc=rpc)
        assert_eq(approved.strip(), "true", f"SP {pid} is approved in FWSS")

        endorsed = _cast(endorsements, _SIG_IS_ENDORSED, addr, rpc=rpc)
        assert_eq(endorsed.strip(), "true", f"SP {pid} ({addr}) is endorsed")


def _check_sp_http(sps: list) -> None:
    """Assert each SP's PDP service URL responds with HTTP 200."""
    for sp in sps:
        pid = sp["provider_id"]
        url = sp["pdp_service_url"].rstrip("/")
        try:
            code = urllib.request.urlopen(url, timeout=5).getcode()
            assert_eq(str(code), "200", f"SP {pid} PDP service HTTP 200 at {url}")
        except Exception as exc:
            fail(f"SP {pid} PDP service unreachable at {url}: {exc}")


def _check_user_storage_readiness(users: list, contracts: dict, rpc: str) -> None:
    """
    Verify USER_1 is ready for FOC warm storage uploads:
      - Has a USDFC balance (can pay for storage)
      - Has credited FIL into FilecoinPay (payment channel funded)
      - Has approved FWSS as a payment operator

    This mirrors the Synapse SDK's preflight checks before calling upload().
    """
    usdfc        = contracts["mockusdfc_addr"]
    filecoin_pay = contracts["filecoin_pay_v1_addr"]
    fwss         = contracts["fwss_service_proxy_addr"]

    storage_users = [u for u in users if u["name"] == "USER_1"]
    if not storage_users:
        fail("USER_1 not found in devnet users; skipping storage readiness checks")
        return

    user = storage_users[0]
    addr = user["evm_addr"]
    name = user["name"]

    # ── 1. USDFC wallet balance ───────────────────────────────────────────────
    raw_balance = sh(
        f"cast call {usdfc} 'balanceOf(address)(uint256)' {addr} --rpc-url {rpc}"
    )
    usdfc_wei = "".join(c for c in raw_balance if c.isdigit())
    assert_gt(usdfc_wei, 0, f"{name} has USDFC wallet balance > 0")

    # ── 2. FilecoinPay deposit (payment channel credited) ────────────────────
    deposit_raw = _cast(filecoin_pay, _SIG_PAYMENT_BALANCE, addr, rpc=rpc)
    deposit_wei = "".join(c for c in deposit_raw if c.isdigit())
    assert_gt(deposit_wei, 0, f"{name} has credited USDFC into FilecoinPay")

    # ── 3. FWSS approved as operator in FilecoinPay ───────────────────────────
    is_op = _cast(filecoin_pay, _SIG_IS_OPERATOR, addr, fwss, rpc=rpc)
    assert_eq(is_op.strip(), "true", f"{name} has FWSS set as FilecoinPay operator")


def run():
    ensure_foundry()
    d        = devnet_info()["info"]
    rpc      = d["lotus"]["host_rpc_url"]
    contracts = d["contracts"]
    users    = d["users"]
    sps      = d.get("pdp_sps", [])

    info("--- Checking service providers ---")
    _check_sps(sps, contracts, rpc)

    info("--- Checking SP PDP HTTP endpoints ---")
    _check_sp_http(sps)

    info("--- Checking USER_1 storage readiness (mirrors Synapse SDK preflight) ---")
    _check_user_storage_readiness(users, contracts, rpc)


if __name__ == "__main__":
    run()
