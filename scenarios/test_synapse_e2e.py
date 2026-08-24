#!/usr/bin/env python3
"""Synapse-driven end-to-end exercise of the deployed FOC system."""

import os, sys  # noqa: E401

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import tempfile
from pathlib import Path

from scenarios.helpers import assert_eq, assert_ok, info, write_random_file
from scenarios.synapse_runtime import prepare_synapse_runtime, run_node_script

RAND_FILE_NAME = "random_file"
RAND_FILE_SIZE = 20 * 1024 * 1024
RAND_FILE_SEED = 42


def run():
    assert_ok("command -v node", "node is installed")

    with tempfile.TemporaryDirectory(prefix="synapse-e2e-") as tmp:
        runtime = prepare_synapse_runtime(Path(tmp))

        random_file = runtime.work_dir / RAND_FILE_NAME
        info(f"Creating random file ({RAND_FILE_SIZE} bytes)")
        write_random_file(random_file, RAND_FILE_SIZE, RAND_FILE_SEED)
        assert_eq(
            random_file.stat().st_size,
            RAND_FILE_SIZE,
            f"{RAND_FILE_NAME} created with exact size {RAND_FILE_SIZE} bytes",
        )

        info("Running the Synapse-driven system E2E against devnet")
        run_node_script(
            runtime,
            "system-e2e.ts",
            "Synapse system E2E",
            args=[str(random_file)],
            env={"NETWORK": "devnet"},
        )


if __name__ == "__main__":
    run()
