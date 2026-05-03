#!/usr/bin/env python3
"""Shared helpers for cloning, building, and uploading via synapse-sdk."""

import os
from pathlib import Path

from scenarios.helpers import info, run_cmd, sh

SYNAPSE_SDK_REPO = "https://github.com/FilOzone/synapse-sdk/"
SYNAPSE_SDK_REF = "8b85290ce100b25a8e431f4c6b1ea0a1eda450ff"


def clone_and_build(tmp_dir: Path) -> Path | None:
    """Clone synapse-sdk into tmp_dir, install deps, build. Returns sdk_dir or None on failure."""
    sdk_dir = tmp_dir / "synapse-sdk"
    if not run_cmd(
        ["git", "clone", SYNAPSE_SDK_REPO, str(sdk_dir)], label="clone synapse-sdk"
    ):
        return None
    if not run_cmd(
        ["git", "checkout", SYNAPSE_SDK_REF],
        cwd=str(sdk_dir),
        label="checkout last known good ref",
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
    run_cmd(
        ["node", "utils/example-storage-e2e.js", str(filepath)],
        cwd=str(sdk_dir),
        env=env,
        label=label,
        print_output=True,
    )
