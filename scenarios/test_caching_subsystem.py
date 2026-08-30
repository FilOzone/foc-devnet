#!/usr/bin/env python3
"""Caching subsystem scenario.

Checks whether uploading a small piece does not trigger caching and
whether a larger piece does trigger caching (> 32MB). Ensures the Scylla
CQL proof-cache (curio.pdp_cache_layer) rows are populated.

Standalone run:
  python3 scenarios/test_caching_subsystem.py
"""

import os, sys  # noqa: E401

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import re
import tempfile
from pathlib import Path

from scenarios.helpers import (
    assert_eq,
    assert_gt,
    assert_ok,
    devnet_info,
    fail,
    info,
    sh,
    write_random_file,
)
from scenarios.synapse_runtime import prepare_synapse_runtime, run_node_script

SMALL_FILE_SIZE = 20 * 1024 * 1024  # 20MB, below the 32MB threshold
LARGE_FILE_SIZE = 80 * 1024 * 1024  # 80MB, above the 32MB threshold
RAND_SEED_SMALL = 42
RAND_SEED_LARGE = 85
RUN_COUNTER_FILE = (
    Path.home() / ".foc-devnet" / "artifacts" / "test_caching_subsystem_counter.integer"
)
CACHE_WAIT_SECS = 10


def _next_run_index() -> int:
    """Read, increment, and persist the run counter so each run uses unique seeds."""
    RUN_COUNTER_FILE.parent.mkdir(parents=True, exist_ok=True)
    try:
        current = int(RUN_COUNTER_FILE.read_text().strip())
    except (FileNotFoundError, ValueError):
        current = 0
    next_index = current + 1
    RUN_COUNTER_FILE.write_text(str(next_index))
    return next_index


def _scylla_container() -> str:
    """Derive the first SP's Scylla container name from devnet-info."""
    dn = devnet_info()["info"]
    provider_id = dn["pdp_sps"][0]["provider_id"]
    return f"foc-{dn['run_id']}-scylla-{provider_id}"


def _cql(scylla_container, query):
    """Run a CQL query via the Scylla container's bundled cqlsh, return raw output."""
    return sh(f'docker exec {scylla_container} cqlsh -e "{query}"')


def _row_count(cql_output):
    """Extract row count from cqlsh output. Calls fail() if pattern not found."""
    match = re.search(r"\((\d+)\s+rows\)", cql_output)
    if match is None:
        fail(f"Could not parse row count from cqlsh output: {cql_output!r}")
    return int(match.group(1))


def _upload_and_count(runtime, filepath, label, scylla_container):
    """Upload a file and return the cache row count afterward."""
    import time

    run_node_script(
        runtime,
        "upload-probe.ts",
        label,
        args=[str(filepath)],
        env={"NETWORK": "devnet"},
    )
    info(f"Waiting {CACHE_WAIT_SECS}s for caching tasks")
    time.sleep(CACHE_WAIT_SECS)
    output = _cql(scylla_container, "SELECT * FROM curio.pdp_cache_layer")
    count = _row_count(output)
    info(f"row_count after '{label}' = {count}")
    return count


def run():
    assert_ok("command -v node", "node is installed")

    run_index = _next_run_index()
    seed_small = RAND_SEED_SMALL + run_index
    seed_large = RAND_SEED_LARGE + run_index
    info(f"Run index: {run_index}, seeds: small={seed_small}, large={seed_large}")

    scylla_container = _scylla_container()
    info(f"Scylla container: {scylla_container}")

    init_output = _cql(scylla_container, "SELECT * FROM curio.pdp_cache_layer")
    init_count = _row_count(init_output)
    info(f"Initial row count = {init_count}")

    with tempfile.TemporaryDirectory(prefix="synapse-cache-") as tmp:
        runtime = prepare_synapse_runtime(Path(tmp))

        small_file = runtime.work_dir / "small_20mb"
        large_file = runtime.work_dir / "large_80mb"
        write_random_file(small_file, SMALL_FILE_SIZE, seed_small)
        write_random_file(large_file, LARGE_FILE_SIZE, seed_large)

        info("Uploading 20MB piece (below 32MB threshold)")
        after_small = _upload_and_count(
            runtime, small_file, "upload 20MB piece", scylla_container
        )
        assert_eq(after_small, init_count, "cache rows count should not increase")

        info("Uploading 80MB piece (above 32MB threshold)")
        after_large = _upload_and_count(
            runtime, large_file, "upload 80MB piece", scylla_container
        )
        assert_gt(after_large, init_count, "cache rows count should increase")


if __name__ == "__main__":
    run()
