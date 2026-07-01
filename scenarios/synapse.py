#!/usr/bin/env python3
"""Shared helpers for cloning, building, and uploading via synapse-sdk."""

from __future__ import annotations

import os
import subprocess
import time
import json
from pathlib import Path

from scenarios.dependencies import component
from scenarios.helpers import fail, info, ok, run_cmd, sh

STATE_FORK_ERROR = "refusing explicit call due to state fork at epoch"
UPLOAD_RETRY_DELAYS_SECS = (5, 10, 15, 30)


def apply_pnpm_workspace_overrides(sdk_dir: Path, overrides: dict) -> bool:
    if not overrides:
        return True

    workspace = sdk_dir / "pnpm-workspace.yaml"
    if not workspace.is_file():
        fail(f"pnpm-workspace.yaml not found at {workspace}")
        return False

    lines = workspace.read_text().splitlines()
    updated = []
    index = 0
    while index < len(lines):
        line = lines[index]
        if line == "overrides:":
            index += 1
            while index < len(lines) and (
                not lines[index] or lines[index].startswith((" ", "#"))
            ):
                index += 1
            continue
        updated.append(line)
        index += 1

    if updated and updated[-1]:
        updated.append("")
    updated.append("overrides:")
    for package, version in sorted(overrides.items()):
        updated.append(f"  {json.dumps(package)}: {json.dumps(version)}")

    workspace.write_text("\n".join(updated) + "\n")
    info(
        "Applied pnpm workspace overrides: "
        + ", ".join(
            f"{package}={version}" for package, version in sorted(overrides.items())
        )
    )
    return True


def clone_and_build(tmp_dir: Path) -> Path | None:
    """Clone synapse-sdk into tmp_dir, install deps, build. Returns sdk_dir or None on failure."""
    dependency = component("synapse-sdk")
    repository = dependency["repository"]
    checkout = dependency.get("commit") or dependency["ref"]
    sdk_dir = tmp_dir / "synapse-sdk"
    if not run_cmd(
        ["git", "clone", repository, str(sdk_dir)], label="clone synapse-sdk"
    ):
        return None
    if not run_cmd(
        ["git", "checkout", "--detach", checkout],
        cwd=str(sdk_dir),
        label=f"checkout synapse-sdk {checkout}",
    ):
        return None
    sdk_commit = sh(f"git -C {sdk_dir} rev-parse HEAD")
    expected_commit = dependency.get("commit")
    if expected_commit and sdk_commit != expected_commit:
        raise RuntimeError(
            f"synapse-sdk checkout is {sdk_commit}, expected {expected_commit}"
        )
    info(f"synapse-sdk commit: {sdk_commit}")
    overrides = dependency.get("overrides", {})
    if not apply_pnpm_workspace_overrides(sdk_dir, overrides):
        return None
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
