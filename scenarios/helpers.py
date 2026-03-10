#!/usr/bin/env python3
"""Shared logging, assertions, shell helpers, and file utilities for scenario tests."""

import json
import os
import random
import subprocess
import sys
from pathlib import Path

_pass = 0
_fail = 0
_log_lines: list = []

DEVNET_INFO = os.environ.get(
    "DEVNET_INFO", os.path.expanduser("~/.foc-devnet/state/latest/devnet-info.json")
)
FOUNDRY_BIN = Path.home() / ".foc-devnet" / "artifacts" / "foundry" / "bin"
CAST = str(FOUNDRY_BIN / "cast")
FORGE = str(FOUNDRY_BIN / "forge")

# ── Logging ──────────────────────────────────────────────────


def info(msg):
    _log_lines.append(f"[INFO] {msg}")
    print(f"[INFO] {msg}")


def ok(msg):
    global _pass
    _log_lines.append(f"[ OK ] {msg}")
    print(f"[ OK ] {msg}")
    _pass += 1


def fail(msg):
    """Logs a failure and exits the scenario entirely with exit code 1."""
    global _fail
    _log_lines.append(f"[FAIL] {msg}")
    print(f"[FAIL] {msg}", file=sys.stderr)
    _fail += 1
    sys.exit(1)


# ── Assertions ───────────────────────────────────────────────


def assert_eq(a, b, msg):
    if a == b:
        ok(msg)
    else:
        fail(f"{msg} (got '{a}', want '{b}')")


def assert_gt(a, b, msg):
    try:
        if int(a) > int(b):
            ok(msg)
        else:
            fail(f"{msg} (got '{a}', want > '{b}')")
    except (TypeError, ValueError):
        fail(f"{msg} (not an int: '{a}')")


def assert_not_empty(v, msg):
    if v:
        ok(msg)
    else:
        fail(f"{msg} (empty)")


def assert_ok(cmd, msg):
    if (
        subprocess.call(
            cmd, shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
        )
        == 0
    ):
        ok(msg)
    else:
        fail(msg)


# ── Shell helpers ─────────────────────────────────────────────


def sh(cmd):
    """Run cmd in a shell and return stdout stripped, or '' on error."""
    return subprocess.run(
        cmd, shell=True, text=True, capture_output=True
    ).stdout.strip()


def run_cmd(
    cmd: list, *, cwd=None, env=None, label: str = "", print_output: bool = False
) -> bool:
    """Run a subprocess command and report pass/fail; returns True on success."""
    result = subprocess.run(cmd, cwd=cwd, env=env, text=True, capture_output=True)
    details = (result.stderr or result.stdout or "").strip()
    if result.returncode == 0:
        if print_output:
            info(details)
        ok(label)
        return True
    fail(f"{label} (exit={result.returncode}) {details}")
    return False


def devnet_info():
    """Load devnet-info.json as a dict."""
    with open(DEVNET_INFO) as f:
        return json.load(f)


# ── File helpers ──────────────────────────────────────────────

_RANDOM_CHUNK_SIZE = 1024 * 1024


def write_random_file(path: Path, size: int, seed: int) -> None:
    """Write a deterministic pseudo-random file of exactly `size` bytes."""
    rng = random.Random(seed)
    remaining = size
    with path.open("wb") as fh:
        while remaining > 0:
            chunk = min(_RANDOM_CHUNK_SIZE, remaining)
            fh.write(rng.randbytes(chunk))
            remaining -= chunk
