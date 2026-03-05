#!/usr/bin/env python3
# Verifies every devnet user has a positive FIL and USDFC balance.
import os, sys

# Ensure the project root (parent of scenarios/) is on sys.path
_project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _project_root not in sys.path:
    sys.path.insert(0, _project_root)

from scenarios.run import *

def run():
    ensure_foundry()
    d = devnet_info()["info"]
    lotus_rpc  = d["lotus"]["host_rpc_url"]
    usdfc_addr = d["contracts"]["mockusdfc_addr"]
    users = d["users"]
    assert_gt(len(users), 0, "at least one user exists")

    for user in users:
        name, user_addr = user["name"], user["evm_addr"]
        fil_wei = sh(f"cast balance {user_addr} --rpc-url {lotus_rpc}")
        assert_gt(fil_wei, 0, f"{name} FIL balance > 0")
        usdfc_raw = sh(f"cast call {usdfc_addr} 'balanceOf(address)(uint256)' {user_addr} --rpc-url {lotus_rpc}")
        usdfc_wei = "".join(c for c in usdfc_raw if c.isdigit())
        assert_gt(usdfc_wei, 0, f"{name} USDFC balance > 0")

if __name__ == "__main__":
    run()
