#!/usr/bin/env python3
"""Verifies all devnet containers are running."""

import os, sys  # noqa: E401

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from scenarios.helpers import assert_eq, devnet_info, sh


def run():
    d = devnet_info()["info"]
    expected = [d["lotus"]["container_name"], d["lotus_miner"]["container_name"]]
    for sp in d.get("pdp_sps", []):
        expected.append(sp["container_name"])

    for name in expected:
        status = sh(
            f"docker inspect -f '{{{{.State.Status}}}}' {name} 2>/dev/null || echo missing"
        )
        assert_eq(status, "running", f"container {name} is running")


if __name__ == "__main__":
    run()
