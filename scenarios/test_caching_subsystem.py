#!/usr/bin/env python3
"""Caching subsystem scenario.

Checks whether uploading a small piece does not trigger caching and
whether a larger piece does trigger caching (> 32MB). Ensures that
cassandra rows are populated.

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
from scenarios.synapse import clone_and_build, upload_file

CASSANDRA_VERSION = "5.0.6"
PYTHON_VERSION = "3.11.10"
PYENV_ROOT = Path.home() / ".pyenv"
PYTHON_DIR = PYENV_ROOT / "versions" / PYTHON_VERSION
CASSANDRA_DIR = Path.home() / ".foc-devnet" / "artifacts" / "cassandra"
CASSANDRA_HOME = CASSANDRA_DIR / f"apache-cassandra-{CASSANDRA_VERSION}"
SMALL_FILE_SIZE = 20 * 1024 * 1024  # 20MB -- below 32MB threshold
LARGE_FILE_SIZE = 80 * 1024 * 1024  # 80MB -- above 32MB threshold
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


def _find_cqlsh():
    """Locate cqlsh and custom Python. Returns (cqlsh_path, python_path)."""
    python_bin = PYTHON_DIR / "bin" / "python3"
    if not python_bin.exists():
        raise RuntimeError(
            f"Python {PYTHON_VERSION} not found at {python_bin}. "
            f"Run scripts/setup-scenarios-prerequisites.sh first."
        )
    cqlsh = CASSANDRA_HOME / "bin" / "cqlsh"
    if not cqlsh.exists():
        raise RuntimeError(
            f"cqlsh not found at {cqlsh}. "
            f"Run scripts/setup-scenarios-prerequisites.sh first."
        )
    info(f"cqlsh version: {sh(f'CQLSH_PYTHON={python_bin} {cqlsh} --version')}")
    return str(cqlsh), str(python_bin)


def _ycql(cqlsh, python, ycql_port, query):
    """Run a YCQL query via cqlsh, return raw output."""
    return sh(
        f'{cqlsh} --python {python} localhost {ycql_port} -u cassandra -p cassandra -e "{query}"'
    )


def _row_count(ycql_output):
    """Extract row count from cqlsh output. Calls fail() if pattern not found."""
    match = re.search(r"\((\d+)\s+rows\)", ycql_output)
    if match is None:
        fail(f"Could not parse row count from cqlsh output: {ycql_output!r}")
    return int(match.group(1))


def _upload_and_count(sdk_dir, filepath, label, cqlsh, python, ycql_port):
    """Upload a file and return the cache row count afterward."""
    import time

    upload_file(sdk_dir, filepath.name, label)
    info(f"Waiting {CACHE_WAIT_SECS}s for caching tasks")
    time.sleep(CACHE_WAIT_SECS)
    output = _ycql(cqlsh, python, ycql_port, "SELECT * FROM curio.pdp_cache_layer")
    count = _row_count(output)
    info(f"row_count after '{label}' = {count}")
    return count


def run():
    assert_ok("command -v git", "git is installed")
    assert_ok("command -v node", "node is installed")
    assert_ok("command -v pnpm", "pnpm is installed")

    run_index = _next_run_index()
    seed_small = RAND_SEED_SMALL + run_index
    seed_large = RAND_SEED_LARGE + run_index
    info(f"Run index: {run_index}, seeds: small={seed_small}, large={seed_large}")

    cqlsh, python = _find_cqlsh()
    ycql_port = devnet_info()["info"]["pdp_sps"][0]["yugabyte"]["ycql_port"]
    info(f"Yugabyte cassandra port: localhost:{ycql_port}")

    init_output = _ycql(cqlsh, python, ycql_port, "SELECT * FROM curio.pdp_cache_layer")
    init_count = _row_count(init_output)
    info(f"Initial row count = {init_count}")

    with tempfile.TemporaryDirectory(prefix="synapse-sdk-cache-") as tmp:
        sdk_dir = clone_and_build(Path(tmp))
        if not sdk_dir:
            return

        small_file = sdk_dir / "small_20mb"
        large_file = sdk_dir / "large_80mb"
        write_random_file(small_file, SMALL_FILE_SIZE, seed_small)
        write_random_file(large_file, LARGE_FILE_SIZE, seed_large)

        info("Uploading 20MB piece (below 32MB threshold)")
        after_small = _upload_and_count(
            sdk_dir, small_file, "upload 20MB piece", cqlsh, python, ycql_port
        )
        assert_eq(after_small, init_count, "cache rows count should not increase")

        info("Uploading 80MB piece (above 32MB threshold)")
        after_large = _upload_and_count(
            sdk_dir, large_file, "upload 80MB piece", cqlsh, python, ycql_port
        )
        assert_gt(after_large, init_count, "cache rows count should increase")


if __name__ == "__main__":
    run()
