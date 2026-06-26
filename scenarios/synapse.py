#!/usr/bin/env python3
"""Shared helpers for cloning, building, and uploading via synapse-sdk."""

import os
import subprocess
import time
from pathlib import Path

from scenarios.helpers import fail, info, ok, run_cmd, sh

STATE_FORK_ERROR = "refusing explicit call due to state fork at epoch"
SYNAPSE_SDK_REPO = "https://github.com/FilOzone/synapse-sdk/"
UPLOAD_RETRY_DELAYS_SECS = (5, 10, 15, 30)


def clone_and_build(tmp_dir: Path) -> Path | None:
    """Clone synapse-sdk into tmp_dir, install deps, build. Returns sdk_dir or None on failure."""
    sdk_dir = tmp_dir / "synapse-sdk"
    if not run_cmd(
        ["git", "clone", SYNAPSE_SDK_REPO, str(sdk_dir)], label="clone synapse-sdk"
    ):
        return None
    if not run_cmd(
        ["git", "checkout", "master"], cwd=str(sdk_dir), label="checkout master HEAD"
    ):
        return None
    sdk_commit = sh(f"git -C {sdk_dir} rev-parse HEAD")
    info(f"synapse-sdk commit: {sdk_commit}")
    if not run_cmd(["pnpm", "install"], cwd=str(sdk_dir), label="pnpm install"):
        return None
    if not run_cmd(["pnpm", "build"], cwd=str(sdk_dir), label="pnpm build"):
        return None
    return sdk_dir


def upload_file(sdk_dir: Path, filepath: str, label: str):
    """Upload a single file via example-storage-e2e.js."""
    env = {**os.environ, "NETWORK": "devnet"}
    cmd = ["node", "utils/example-storage-e2e.js", str(filepath)]
    max_attempts = len(UPLOAD_RETRY_DELAYS_SECS) + 1

    for attempt in range(1, max_attempts + 1):
        result = subprocess.run(
            cmd,
            cwd=str(sdk_dir),
            env=env,
            text=True,
            capture_output=True,
        )
        details = "\n".join(
            part for part in (result.stderr.strip(), result.stdout.strip()) if part
        )
        if result.returncode == 0:
            if details:
                info(details)
            ok(label)
            return

        if STATE_FORK_ERROR not in details or attempt == max_attempts:
            fail(f"{label} (exit={result.returncode}) {details}")

        delay = UPLOAD_RETRY_DELAYS_SECS[attempt - 1]
        info(
            f"{label}: Lotus refused eth_call while crossing a state fork; "
            f"retrying in {delay}s (attempt {attempt}/{max_attempts})"
        )
        time.sleep(delay)
