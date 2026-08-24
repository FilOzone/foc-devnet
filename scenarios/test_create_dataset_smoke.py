#!/usr/bin/env python3
"""Smoke test for the PDP/FWSS createDataSet path via synapse-sdk."""

import os, sys  # noqa: E401

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

import tempfile
from pathlib import Path

from scenarios.helpers import assert_ok, info
from scenarios.synapse_runtime import prepare_synapse_runtime, run_node_script

SMOKE_TIMEOUT_SECS = 280
SMOKE_USER_INDEX = "1"  # USER_2; USER_1 is used by existing storage scenarios.


def run():
    assert_ok("command -v node", "node is installed")

    with tempfile.TemporaryDirectory(prefix="synapse-createdataset-") as tmp:
        runtime = prepare_synapse_runtime(Path(tmp))

        info("Running createDataSet smoke script against devnet")
        run_node_script(
            runtime,
            "create-dataset.ts",
            "createDataSet smoke test",
            env={"DEVNET_USER_INDEX": SMOKE_USER_INDEX},
            timeout=SMOKE_TIMEOUT_SECS,
        )


if __name__ == "__main__":
    run()
