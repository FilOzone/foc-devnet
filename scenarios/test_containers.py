#!/usr/bin/env python3
# Verifies all devnet containers are running and no unexpected foc-* containers exist.
import os, sys

# Ensure the project root (parent of scenarios/) is on sys.path
_project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _project_root not in sys.path:
    sys.path.insert(0, _project_root)

from scenarios.run import *

def run():
    d = devnet_info()["info"]
    run_id = d.get("run_id", "")

    expected = [d["lotus"]["container_name"], d["lotus_miner"]["container_name"]]
    sps = d.get("pdp_sps", [])
    for sp in sps:
        expected.append(sp["container_name"])

    for name in expected:
        status = sh(f"docker inspect -f '{{{{.State.Status}}}}' {name} 2>/dev/null || echo missing")
        assert_eq(status, "running", f"container {name} is running")

if __name__ == "__main__":
    run()
