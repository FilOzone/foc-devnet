#!/usr/bin/env python3
"""End-to-end storage test: upload a random file via synapse-sdk against the devnet."""

import os, sys  # noqa: E401

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import tempfile
from pathlib import Path

from scenarios.helpers import assert_eq, assert_ok, info, write_random_file
from scenarios.synapse import clone_and_build, upload_file

RAND_FILE_NAME = "random_file"
RAND_FILE_SIZE = 20 * 1024 * 1024
RAND_FILE_SEED = 42


def run():
    assert_ok("command -v git", "git is installed")
    assert_ok("command -v node", "node is installed")
    assert_ok("command -v pnpm", "pnpm is installed")

    with tempfile.TemporaryDirectory(prefix="synapse-sdk-") as tmp:
        sdk_dir = clone_and_build(Path(tmp))
        if not sdk_dir:
            return

        random_file = sdk_dir / RAND_FILE_NAME
        info(f"Creating random file ({RAND_FILE_SIZE} bytes)")
        write_random_file(random_file, RAND_FILE_SIZE, RAND_FILE_SEED)
        assert_eq(
            random_file.stat().st_size,
            RAND_FILE_SIZE,
            f"{RAND_FILE_NAME} created with exact size {RAND_FILE_SIZE} bytes",
        )

        info("Running Synapse SDK storage e2e script against devnet")
        upload_file(
            sdk_dir,
            RAND_FILE_NAME,
            "NETWORK=devnet node utils/example-storage-e2e.js random_file",
        )


if __name__ == "__main__":
    run()
