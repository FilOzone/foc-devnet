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
import re
import random
import tempfile
from pathlib import Path

_project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _project_root not in sys.path:
    sys.path.insert(0, _project_root)

from scenarios.run import *

SYNAPSE_SDK_REPO = "https://github.com/FilOzone/synapse-sdk/"
CASSANDRA_VERSION = "5.0.6"
PYTHON_VERSION = "3.10"  # max supported python version for CASSANDRA_VERSION
CASSANDRA_URL = f"https://dlcdn.apache.org/cassandra/{CASSANDRA_VERSION}/apache-cassandra-{CASSANDRA_VERSION}-bin.tar.gz"
CASSANDRA_DIR = Path.home() / ".foc-devnet" / "artifacts" / "cassandra"
CASSANDRA_HOME = CASSANDRA_DIR / f"apache-cassandra-{CASSANDRA_VERSION}"
SMALL_FILE_SIZE = 20 * 1024 * 1024  # 20MB — below 32MB threshold
LARGE_FILE_SIZE = 80 * 1024 * 1024  # 80MB — above 32MB threshold
# Base seeds for pseudo-random file generation.  The actual seeds used in each
# run are offset by the run index (see _next_run_index) so that successive runs
# always produce different byte content and therefore different pieceCIDs.
RAND_SEED_SMALL = 42
RAND_SEED_LARGE = 85
# Path to the persistent run counter.  A plain integer is stored here so that
# every invocation of this test uses a unique seed offset.
RUN_COUNTER_FILE = (
    Path.home() / ".foc-devnet" / "artifacts" / "test_caching_subsystem_counter.integer"
)
CACHE_WAIT_SECS = 10
GOCQL_ERROR = "gocql: no hosts available in the pool"
_CHUNK = 1024 * 1024


def _next_run_index() -> int:
    """Read, increment, and persist the run counter stored in RUN_COUNTER_FILE.

    The counter starts at 1 on the very first call (file absent or empty).
    Incrementing before returning ensures the on-disk value always reflects
    the *current* run, so a crash after writing files but before uploading
    will still advance the counter on retry.
    """
    # Create parent directory in case this is the very first run.
    RUN_COUNTER_FILE.parent.mkdir(parents=True, exist_ok=True)
    try:
        current = int(RUN_COUNTER_FILE.read_text().strip())
    except (FileNotFoundError, ValueError):
        # File absent or corrupt — treat as run 0 so the first real run is 1.
        current = 0
    next_index = current + 1
    RUN_COUNTER_FILE.write_text(str(next_index))
    return next_index


def _write_random_file(path: Path, size: int, seed: int) -> None:
    """Write a deterministic pseudo-random file of exactly `size` bytes."""
    rng = random.Random(seed)
    remaining = size
    with path.open("wb") as fh:
        while remaining > 0:
            chunk = min(_CHUNK, remaining)
            fh.write(rng.randbytes(chunk))
            remaining -= chunk


def _ensure_custom_python():
    """Ensure python is available on the system, installing it via apt if needed.
    This is because cassandra 5.0.6 requires Python 3.6-3.11

    Returns the path to the python3.11 interpreter.
    """
    from pathlib import Path

    npm_root = sh("npm root -g").strip()
    pkg_dir = Path(npm_root) / "@bjia56" / f"portable-python-{PYTHON_VERSION}"
    py = ""
    if pkg_dir.exists():
        # look for any python binary under the package (handles headless builds)
        for p in pkg_dir.rglob("bin/python*"):
            py = str(p)
            break

    if not py or py == "":
        info(f"Installing custom python version {PYTHON_VERSION}")
        info(sh(f"npm i --global --silent @bjia56/portable-python-{PYTHON_VERSION}"))

    if pkg_dir.exists():
        # look for any python binary under the package (handles headless builds)
        for p in pkg_dir.rglob("bin/python*"):
            py = str(p)
            break

    if not py or py == "":
        raise RuntimeError(
            f"python {PYTHON_VERSION} not found after installation attempt"
        )
    else:
        info(f"Found custom python @ {py}")

    return py


def _install_cqlsh():
    """Download Apache Cassandra tarball and return (cqlsh_path, python3.11_path).

    The tarball is extracted once to ~/.foc-devnet/artifacts/cassandra/ and reused
    on subsequent runs. Custom Python is ensured before returning.
    """
    custom_python = _ensure_custom_python()
    cqlsh = CASSANDRA_HOME / "bin" / "cqlsh"
    if cqlsh.exists():
        cqlsh_version = sh(f"CQLSH_PYTHON={custom_python} {cqlsh} --version")
        info(f"cqlsh version chk: {cqlsh_version}")
        info(f"cqlsh already installed at {cqlsh}")
        return str(cqlsh), custom_python
    info("--- Installing cqlsh from Apache Cassandra tarball ---")
    CASSANDRA_DIR.mkdir(parents=True, exist_ok=True)
    tarball = CASSANDRA_DIR / f"apache-cassandra-{CASSANDRA_VERSION}-bin.tar.gz"
    sh(f"curl -fL -o {tarball} {CASSANDRA_URL}")
    sh(f"tar -xzf {tarball} -C {CASSANDRA_DIR}")
    cqlsh_version = sh(f"CQLSH_PYTHON={custom_python} {cqlsh} --version")
    info(f"cqlsh version chk: {cqlsh_version}")
    return str(cqlsh), custom_python


def _ycql(cqlsh, python, ycql_port, query):
    """Run a YCQL query on the host via cqlsh using the given Python interpreter, return raw output."""
    return sh(
        f'{cqlsh} --python {python} localhost {ycql_port} -u cassandra -p cassandra -e "{query}"'
    )


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


def _row_count(ycql_output):
    return int(re.search(r"\((\d+)\s+rows\)", ycql_output).group(1))


def run():
    assert_ok("command -v git", "git is installed")
    assert_ok("command -v node", "node is installed")
    assert_ok("command -v pnpm", "pnpm is installed")

    # Advance the persistent run counter so each execution uses a unique seed
    # offset.  This guarantees different file content → different pieceCID every
    # run, preventing the node from deduplicating uploads across test runs.
    run_index = _next_run_index()
    seed_small = RAND_SEED_SMALL + run_index
    seed_large = RAND_SEED_LARGE + run_index
    info(f"Run index: {run_index} (persisted to {RUN_COUNTER_FILE})")
    info(f"Effective seeds — small file: {seed_small}, large file: {seed_large}")

    cqlsh, python = _install_cqlsh()

    d = devnet_info()["info"]
    sp = d["pdp_sps"][0]
    yb = sp["yugabyte"]
    ycql_port = yb["ycql_port"]

    info(f"Yugabyte cassandra port: localhost:{ycql_port}")
    init_ycql_output = _ycql(
        cqlsh, python, ycql_port, "SELECT * FROM curio.pdp_cache_layer"
    )
    init_ycql_row_count = _row_count(init_ycql_output)
    info(init_ycql_output)
    info(f"init_ycql_row_count = {init_ycql_row_count}")

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
        large_file = sdk_dir / "large_80mb"
        # Use the run-specific seeds so pieceCIDs differ from previous runs.
        info(
            f"Writing small file ({SMALL_FILE_SIZE // (1024*1024)}MB) with seed {seed_small}"
        )
        _write_random_file(small_file, SMALL_FILE_SIZE, seed_small)
        info(
            f"Writing large file ({LARGE_FILE_SIZE // (1024*1024)}MB) with seed {seed_large}"
        )
        _write_random_file(large_file, LARGE_FILE_SIZE, seed_large)

        info(" Uploading 20MB piece (below 32MB threshold)")
        _upload_file(sdk_dir, small_file.name, "upload 20MB piece")
        info(f"Waiting {CACHE_WAIT_SECS}s for caching tasks")
        time.sleep(CACHE_WAIT_SECS)
        after_20mb_upload = _ycql(
            cqlsh, python, ycql_port, "SELECT * FROM curio.pdp_cache_layer"
        )
        after_20mb_row_count = _row_count(after_20mb_upload)
        info(after_20mb_upload)
        info(f"after_20mb_upload_row_count = {after_20mb_row_count}")
        assert_eq(
            after_20mb_row_count,
            init_ycql_row_count,
            "cache rows count should not increase",
        )

        info(" Uploading 80MB piece (below 32MB threshold)")
        _upload_file(sdk_dir, large_file.name, "upload 80MB piece")
        info(f"Waiting {CACHE_WAIT_SECS}s for caching tasks")
        time.sleep(CACHE_WAIT_SECS)
        after_80mb_upload = _ycql(
            cqlsh, python, ycql_port, "SELECT * FROM curio.pdp_cache_layer"
        )
        after_80mb_row_count = _row_count(after_80mb_upload)
        info(after_80mb_upload)
        info(f"after_80mb_upload_row_count = {after_80mb_row_count}")
        assert_gt(
            after_80mb_row_count,
            init_ycql_row_count,
            "cache rows count should increase",
        )


if __name__ == "__main__":
    run()
