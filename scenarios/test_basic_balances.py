#!/usr/bin/env python3
"""Verifies every devnet user has a positive FIL and USDFC balance."""

import os, sys  # noqa: E401

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from scenarios.helpers import CAST, assert_gt, assert_ok, devnet_info, sh


def run():
    assert_ok(f"test -x {CAST}", "cast is installed")
    d = devnet_info()["info"]
    lotus_rpc = d["lotus"]["host_rpc_url"]
    usdfc_addr = d["contracts"]["mockusdfc_addr"]
    users = d["users"]
    assert_gt(len(users), 0, "at least one user exists")

    for user in users:
        name, user_addr = user["name"], user["evm_addr"]
        fil_wei = sh(f"{CAST} balance {user_addr} --rpc-url {lotus_rpc}")
        assert_gt(fil_wei, 0, f"{name} FIL balance > 0")
        usdfc_raw = sh(
            f"{CAST} call {usdfc_addr} 'balanceOf(address)(uint256)' {user_addr} "
            f"--from {user_addr} --rpc-url {lotus_rpc}"
        )
        usdfc_wei = "".join(c for c in usdfc_raw if c.isdigit())
        assert_gt(usdfc_wei, 0, f"{name} USDFC balance > 0")


if __name__ == "__main__":
    run()
