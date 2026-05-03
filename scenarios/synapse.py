#!/usr/bin/env python3
"""Shared helpers for cloning, building, waiting for FWSS readiness, and uploading via synapse-sdk."""

import os
import subprocess
import time
from pathlib import Path

from scenarios.helpers import CAST, devnet_info, fail, info, ok, run_cmd, sh

SYNAPSE_SDK_REPO = "https://github.com/FilOzone/synapse-sdk/"


def _first_user_evm_addr(users) -> str:
    """Support both list-shaped and dict-shaped devnet-info user sections."""
    if isinstance(users, list):
        return users[0]["evm_addr"]

    # Be forgiving in case the schema uses USER_1 or user_1.
    for key in ("USER_1", "user_1", "user1"):
        if key in users:
            return users[key]["evm_addr"]

    # Last-resort fallback: first dict value.
    return next(iter(users.values()))["evm_addr"]


def _devnet_contract_readiness_values():
    """Extract the exact values needed to call getClientDataSets(USER_1, 0, 100)."""
    raw = devnet_info()
    d = raw.get("info", raw)

    contracts = d["contracts"]
    users = d["users"]
    lotus = d["lotus"]

    rpc_url = (
        os.environ.get("RPC_URL") or lotus.get("host_rpc_url") or lotus.get("rpc_url")
    )

    if not rpc_url:
        fail("Could not find Lotus RPC URL in devnet-info.json")

    fwss_view = (
        os.environ.get("FWSS_STATE_VIEW_ADDR")
        or os.environ.get("WARM_STORAGE_VIEW_ADDRESS")
        or contracts["fwss_state_view_addr"]
    )

    user_1 = (
        os.environ.get("USER_1_EVM_ADDR")
        or os.environ.get("USER_1_ADDRESS")
        or _first_user_evm_addr(users)
    )

    return rpc_url, fwss_view, user_1


def wait_for_get_client_data_sets_ready(
    *,
    timeout_secs: int | None = None,
    interval_secs: int | None = None,
) -> None:
    """
    Wait until this exact read succeeds:

        getClientDataSets(USER_1, 0, 100)

    This catches the fresh-devnet PDP/FWSS condition before Synapse SDK starts
    provider selection and fails with a large viem stack trace.
    """
    timeout_secs = timeout_secs or int(
        os.environ.get("FOC_FWSS_READY_TIMEOUT_SECS", "120")
    )
    interval_secs = interval_secs or int(
        os.environ.get("FOC_FWSS_READY_INTERVAL_SECS", "5")
    )

    rpc_url, fwss_view, user_1 = _devnet_contract_readiness_values()

    cmd = [
        CAST,
        "call",
        fwss_view,
        "getClientDataSets(address,uint256,uint256)",
        user_1,
        "0",
        "100",
        "--rpc-url",
        rpc_url,
    ]

    info(
        "Waiting for FWSS getClientDataSets(USER_1, 0, 100) to succeed "
        f"(timeout={timeout_secs}s, interval={interval_secs}s)"
    )
    info(f"FWSS view: {fwss_view}")
    info(f"USER_1: {user_1}")
    info(f"RPC URL: {rpc_url}")

    deadline = time.time() + timeout_secs
    attempt = 0
    last_output = ""

    while time.time() < deadline:
        attempt += 1

        result = subprocess.run(
            cmd,
            text=True,
            capture_output=True,
        )

        if result.returncode == 0:
            ok("FWSS getClientDataSets(USER_1, 0, 100) succeeds")
            return

        last_output = (result.stderr or result.stdout or "").strip()

        # Avoid spamming logs every 5 seconds, but give enough signal.
        if attempt == 1 or attempt % max(1, 30 // interval_secs) == 0:
            info(
                "FWSS getClientDataSets is not ready yet "
                f"(attempt={attempt}, exit={result.returncode})"
            )
            if last_output:
                info(last_output[-1200:])

        time.sleep(interval_secs)

    hint = ""
    if "RetCode=33" in last_output or "Proving" in last_output:
        hint = (
            "\n\nLikely cause: FWSS/PDP proving-period state is not initialized yet. "
            "This is the same class of failure as Synapse SDK provider selection "
            "reverting during getClientDataSets(...)."
        )

    fail(
        "Timed out waiting for FWSS getClientDataSets(USER_1, 0, 100) to succeed."
        f"{hint}\n\nLast output:\n{last_output}"
    )


def clone_and_build(tmp_dir: Path) -> Path | None:
    """Clone synapse-sdk into tmp_dir, install deps, build.

    Returns sdk_dir or None on failure.
    """
    sdk_dir = tmp_dir / "synapse-sdk"

    if not run_cmd(
        ["git", "clone", SYNAPSE_SDK_REPO, str(sdk_dir)],
        label="clone synapse-sdk",
    ):
        return None

    if not run_cmd(
        ["git", "checkout", "master"],
        cwd=str(sdk_dir),
        label="checkout master HEAD",
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
    wait_for_get_client_data_sets_ready()

    env = {**os.environ, "NETWORK": "devnet"}

    run_cmd(
        ["node", "utils/example-storage-e2e.js", str(filepath)],
        cwd=str(sdk_dir),
        env=env,
        label=label,
        print_output=True,
    )
