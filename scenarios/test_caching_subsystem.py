#!/usr/bin/env python3
"""
Caching subsystem scenario.

Checks whether uploading a small piece does not trigger caching and
whether a larger piece does trigger caching (> 32MB). Ensures that
cassandra rows are populated.

Standalone run:
  python3 scenarios/test_caching_subsystem.py
"""

import os
import sys
import time
import random
import tempfile
from pathlib import Path

_project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _project_root not in sys.path:
    sys.path.insert(0, _project_root)

from scenarios.run import *

SYNAPSE_SDK_REPO = "https://github.com/FilOzone/synapse-sdk/"
SMALL_FILE_SIZE = 20 * 1024 * 1024  # 20MB — below 32MB threshold
LARGE_FILE_SIZE = 60 * 1024 * 1024  # 60MB — above 32MB threshold
RAND_SEED_SMALL = 42
RAND_SEED_LARGE = 84
CACHE_WAIT_SECS = 10
GOCQL_ERROR = "gocql: no hosts available in the pool"
_CHUNK = 1024 * 1024


def _write_random_file(path: Path, size: int, seed: int) -> None:
    """Write a deterministic pseudo-random file of exactly `size` bytes."""
    rng = random.Random(seed)
    remaining = size
    with path.open("wb") as fh:
        while remaining > 0:
            chunk = min(_CHUNK, remaining)
            fh.write(rng.randbytes(chunk))
            remaining -= chunk


def _install_cqlsh(venv_dir):
    """Install cqlsh into a temporary venv, return path to cqlsh binary."""
    cqlsh = os.path.join(venv_dir, "bin", "cqlsh")
    info("--- Installing cqlsh into temp venv ---")
    sh(f"python3 -m venv {venv_dir}")
    sh(f"{venv_dir}/bin/pip install cqlsh")
    assert_ok(f"test -x {cqlsh}", "cqlsh installed")
    return cqlsh


def _ycql(cqlsh, ycql_port, query):
    """Run a YCQL query on the host via cqlsh, return raw output."""
    return sh(f'{cqlsh} localhost {ycql_port} -u cassandra -p cassandra -e "{query}"')


def _upload_file(sdk_dir, filepath, label):
    """Upload a single file via example-storage-e2e.js."""
    env = {**os.environ, "NETWORK": "devnet"}
    run_cmd(
        ["node", "utils/example-storage-e2e.js", str(filepath)],
        cwd=str(sdk_dir),
        env=env,
        label=label,
        print_output=True,
    )


def _verify_cache_layer(cqlsh, ycql_port, expected_is_empty=True):
    """Check pdp_cache_layer is empty due to gocql connectivity issue."""
    info("--- Querying pdp_cache_layer ---")
    out = _ycql(cqlsh, ycql_port, "SELECT * FROM curio.pdp_cache_layer")
    info(f"CQL SELECT access: \n {out}")
    actual_is_empty = "(0 rows)" in out
    assert_eq(actual_is_empty, expected_is_empty, "ysql row count")


def run():
    assert_ok("command -v git", "git is installed")
    assert_ok("command -v node", "node is installed")
    assert_ok("command -v pnpm", "pnpm is installed")

    d = devnet_info()["info"]
    sp = d["pdp_sps"][0]
    yb = sp["yugabyte"]
    ycql_port = yb["ycql_port"]

    with tempfile.TemporaryDirectory(prefix="cqlsh-venv-") as venv_dir:
        cqlsh = _install_cqlsh(venv_dir)

        with tempfile.TemporaryDirectory(prefix="synapse-sdk-cache-") as tmp:
            sdk_dir = Path(tmp) / "synapse-sdk"
            info("--- Cloning synapse-sdk ---")
            if not run_cmd(
                ["git", "clone", SYNAPSE_SDK_REPO, str(sdk_dir)],
                label="clone synapse-sdk",
            ):
                return
            if not run_cmd(
                ["git", "checkout", "master"],
                cwd=str(sdk_dir),
                label="checkout master HEAD",
            ):
                return
            if not run_cmd(["pnpm", "install"], cwd=str(sdk_dir), label="pnpm install"):
                return
            if not run_cmd(["pnpm", "build"], cwd=str(sdk_dir), label="pnpm build"):
                return

            small_file = sdk_dir / "small_20mb"
            large_file = sdk_dir / "large_60mb"
            _write_random_file(small_file, SMALL_FILE_SIZE, RAND_SEED_SMALL)
            _write_random_file(large_file, LARGE_FILE_SIZE, RAND_SEED_LARGE)

            info("--- Uploading 20MB piece (below 32MB threshold) ---")
            _upload_file(sdk_dir, small_file.name, "upload 20MB piece")
            info(f"--- Waiting {CACHE_WAIT_SECS}s for caching tasks ---")
            time.sleep(CACHE_WAIT_SECS)
            _verify_cache_layer(cqlsh, ycql_port, expected_is_empty=True)

            info("--- Uploading 60MB piece (above 32MB threshold) ---")
            _upload_file(sdk_dir, large_file.name, "upload 60MB piece")
            time.sleep(CACHE_WAIT_SECS)
            _verify_cache_layer(cqlsh, ycql_port, expected_is_empty=False)


if __name__ == "__main__":
    run()
