#!/usr/bin/env python3
"""Smoke test for the PDP/FWSS createDataSet path via synapse-sdk."""

import os, sys  # noqa: E401

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import shutil
import tempfile
from pathlib import Path

from scenarios.helpers import assert_ok, info
from scenarios.synapse import clone_and_build, run_node_script

SMOKE_TIMEOUT_SECS = 280
SMOKE_USER_INDEX = "1"  # USER_2; USER_1 is used by existing storage scenarios.
SMOKE_SCRIPT_SOURCE = Path(__file__).with_name("create-dataset-smoke.ts")


def run():
    assert_ok("command -v git", "git is installed")
    assert_ok("command -v node", "node is installed")
    assert_ok("command -v pnpm", "pnpm is installed")

    with tempfile.TemporaryDirectory(prefix="synapse-sdk-createdataset-") as tmp:
        sdk_dir = clone_and_build(Path(tmp))
        if not sdk_dir:
            return

        script_path = sdk_dir / "utils" / SMOKE_SCRIPT_SOURCE.name
        shutil.copyfile(SMOKE_SCRIPT_SOURCE, script_path)

        info("Running createDataSet smoke script against devnet")
        run_node_script(
            sdk_dir,
            script_path,
            "createDataSet smoke test",
            env={"DEVNET_USER_INDEX": SMOKE_USER_INDEX},
            timeout=SMOKE_TIMEOUT_SECS,
        )


if __name__ == "__main__":
    run()
