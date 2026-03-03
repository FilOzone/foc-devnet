#!/usr/bin/env python3
# Verifies all devnet containers are running and no unexpected foc-* containers exist.
from scenarios_py.run import *

def run():
    d = devnet_info()["info"]
    run_id = d.get("run_id", "")

    expected = [d["lotus"]["container_name"], d["lotus_miner"]["container_name"]]
    sps = d.get("pdp_sps", [])
    for sp in sps:
        expected.append(sp["container_name"])
    for i in range(1, len(sps) + 1):
        expected.append(f"foc-{run_id}-yugabyte-{i}")

    for name in expected:
        status = sh(f"docker inspect -f '{{{{.State.Status}}}}' {name} 2>/dev/null || echo missing")
        assert_eq(status, "running", f"container {name} is running")

    prefix = f"foc-{run_id}-" if run_id else "foc-"
    running = sh(f"docker ps --filter name={prefix} --format '{{{{.Names}}}}'").split()
    for name in running:
        known = name in expected or "-portainer" in name
        assert_eq(known, True, f"container {name} is expected")

if __name__ == "__main__":
    run()
