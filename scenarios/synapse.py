#!/usr/bin/env python3
"""Shared helpers for cloning, building, and uploading via synapse-sdk."""

import json
import os
import subprocess
import time
import urllib.error
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

from scenarios.helpers import devnet_info, fail, info, ok, run_cmd, sh

GET_PROVIDER_IDS_SELECTOR = "0x0a9cb4a7"
RPC_TIMEOUT_SECS = 10
SYNAPSE_SDK_REPO = "https://github.com/FilOzone/synapse-sdk/"


def _compact(value, max_len: int = 500) -> str:
    """Return a single-line representation suitable for scenario logs."""
    if not isinstance(value, str):
        value = json.dumps(value, sort_keys=True)
    value = value.replace("\n", "\\n")
    if len(value) > max_len:
        return f"{value[:max_len]}..."
    return value


def _rpc_request(url: str, method: str, params: list | None = None) -> dict:
    payload = json.dumps(
        {"jsonrpc": "2.0", "method": method, "params": params or [], "id": 1}
    ).encode()
    request = urllib.request.Request(
        url,
        data=payload,
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=RPC_TIMEOUT_SECS) as response:
            body = response.read().decode()
    except urllib.error.HTTPError as exc:
        body = exc.read().decode(errors="replace")
        return {"transport_error": f"HTTP {exc.code}: {body}"}
    except (urllib.error.URLError, TimeoutError) as exc:
        return {"transport_error": str(exc)}

    try:
        return json.loads(body)
    except json.JSONDecodeError:
        return {"transport_error": f"invalid JSON response: {body}"}


def _log_chain_head(url: str, label: str) -> None:
    response = _rpc_request(url, "Filecoin.ChainHead")
    if "transport_error" in response:
        info(
            f"RPC diagnostics ({label}): ChainHead transport error: "
            f"{_compact(response['transport_error'])}"
        )
        return
    if "error" in response:
        info(
            f"RPC diagnostics ({label}): ChainHead error: "
            f"{_compact(response['error'])}"
        )
        return

    result = response.get("result") or {}
    height = result.get("Height")
    cids = [
        cid.get("/") for cid in result.get("Cids", []) if isinstance(cid, dict)
    ]
    info(f"RPC diagnostics ({label}): ChainHead height={height} cids={cids}")


def _log_eth_block_number(url: str, label: str) -> None:
    response = _rpc_request(url, "eth_blockNumber")
    if "transport_error" in response:
        info(
            f"RPC diagnostics ({label}): eth_blockNumber transport error: "
            f"{_compact(response['transport_error'])}"
        )
        return
    if "error" in response:
        info(
            f"RPC diagnostics ({label}): eth_blockNumber error: "
            f"{_compact(response['error'])}"
        )
        return

    block_hex = response.get("result")
    try:
        block_number = int(block_hex, 16)
    except (TypeError, ValueError):
        block_number = "unknown"
    info(
        f"RPC diagnostics ({label}): eth_blockNumber={block_number} "
        f"raw={block_hex}"
    )


def _log_get_provider_ids(url: str, data: dict, label: str) -> None:
    info_data = data.get("info", {})
    users = info_data.get("users", [])
    contracts = info_data.get("contracts", {})
    from_addr = users[0].get("evm_addr") if users else None
    to_addr = contracts.get("endorsements_addr")
    if not (from_addr and to_addr):
        info(
            f"RPC diagnostics ({label}): cannot call getProviderIds; "
            "missing user or endorsements address"
        )
        return

    call = {"from": from_addr, "to": to_addr, "data": GET_PROVIDER_IDS_SELECTOR}
    info(
        f"RPC diagnostics ({label}): eth_call getProviderIds "
        f"from={from_addr} to={to_addr} data={GET_PROVIDER_IDS_SELECTOR}"
    )
    response = _rpc_request(url, "eth_call", [call, "latest"])
    if "transport_error" in response:
        info(
            f"RPC diagnostics ({label}): getProviderIds transport error: "
            f"{_compact(response['transport_error'])}"
        )
        return
    if "error" in response:
        info(
            f"RPC diagnostics ({label}): getProviderIds error: "
            f"{_compact(response['error'])}"
        )
        return
    info(
        f"RPC diagnostics ({label}): getProviderIds result="
        f"{_compact(response.get('result'))}"
    )


def _log_rpc_diagnostics(label: str) -> None:
    timestamp = datetime.now(timezone.utc).isoformat()
    info(f"RPC diagnostics ({label}) at {timestamp}")
    try:
        data = devnet_info()
    except Exception as exc:
        info(f"RPC diagnostics ({label}): unable to load devnet-info: {exc}")
        return

    url = data.get("info", {}).get("lotus", {}).get("host_rpc_url")
    if not url:
        info(f"RPC diagnostics ({label}): missing Lotus RPC URL in devnet-info")
        return

    info(f"RPC diagnostics ({label}): lotus_rpc_url={url}")
    _log_chain_head(url, label)
    _log_eth_block_number(url, label)
    _log_get_provider_ids(url, data, label)


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
    _log_rpc_diagnostics(f"before {label}")
    result = subprocess.run(
        cmd,
        cwd=str(sdk_dir),
        env=env,
        text=True,
        capture_output=True,
    )
    details = (result.stderr or result.stdout or "").strip()
    if result.returncode == 0:
        if details:
            info(details)
        _log_rpc_diagnostics(f"after {label}")
        ok(label)
        return

    _log_rpc_diagnostics(f"after failed {label}")
    info(f"RPC diagnostics ({label}): waiting 5s before retry")
    time.sleep(5)
    _log_rpc_diagnostics(f"retry after failed {label}")
    fail(f"{label} (exit={result.returncode}) {details}")
