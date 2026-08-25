#!/usr/bin/env python3
"""Prepare and run the Synapse E2E consumer or source runtime."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path

from scenarios.dependencies import component
from scenarios.helpers import fail, info, ok, run_cmd

STATE_FORK_ERROR = "refusing explicit call due to state fork at epoch"
UPLOAD_RETRY_DELAYS_SECS = (5, 10, 15, 30)
RUNTIME_MARKER = ".synapse-runtime.json"


@dataclass(frozen=True)
class SynapseRuntime:
    work_dir: Path
    source: str
    provenance: str
    source_dir: Path | None = None


def _scenario_dir() -> Path:
    return Path(__file__).with_name("synapse-e2e")


def _runtime_marker(work_dir: Path) -> Path:
    return work_dir / RUNTIME_MARKER


def _npm_view(package: str, version: str, *fields: str):
    result = subprocess.run(
        ["npm", "view", f"{package}@{version}", *fields, "--json"],
        text=True,
        capture_output=True,
    )
    if result.returncode:
        raise RuntimeError(
            f"npm metadata lookup failed for {package}@{version}: {result.stderr.strip()}"
        )
    value = json.loads(result.stdout)
    if fields == ("version",):
        return _npm_version(value, package, version)
    return value


def _npm_version(value, package: str, requested: str) -> str:
    if isinstance(value, str) and value:
        return value
    if isinstance(value, list):
        for version in reversed(value):
            if isinstance(version, str) and version:
                return version
    raise RuntimeError(f"npm returned no version for {package}@{requested}")


def _npm_runtime_dependencies(dependency: dict) -> dict[str, str]:
    metadata = _npm_view(
        dependency["package"], dependency["version"], "dependencies", "peerDependencies"
    )
    core_range = metadata.get("dependencies", {}).get("@filoz/synapse-core")
    viem_range = metadata.get("peerDependencies", {}).get("viem") or metadata.get(
        "dependencies", {}
    ).get("viem")
    if not isinstance(core_range, str) or not isinstance(viem_range, str):
        raise RuntimeError(
            f"{dependency['package']}@{dependency['version']} must declare "
            "@filoz/synapse-core and viem"
        )
    return {
        "@filoz/synapse-core": _npm_version(
            _npm_view("@filoz/synapse-core", core_range, "version"),
            "@filoz/synapse-core",
            core_range,
        ),
        "viem": _npm_version(
            _npm_view("viem", viem_range, "version"), "viem", viem_range
        ),
    }


def _write_manifest(work_dir: Path, dependency: dict) -> None:
    runtime_dependencies = dependency.get("runtime_dependencies")
    if not isinstance(runtime_dependencies, dict):
        runtime_dependencies = _npm_runtime_dependencies(dependency)

    dependencies = {
        dependency["package"]: dependency["version"],
        **runtime_dependencies,
    }
    overrides = {
        package: spec["version"]
        for package, spec in dependency.get("overrides", {}).items()
    }
    manifest = {
        "name": "foc-devnet-synapse-e2e",
        "private": True,
        "type": "module",
        "dependencies": dependencies,
    }
    if overrides:
        manifest["overrides"] = overrides
    (work_dir / "package.json").write_text(json.dumps(manifest, indent=2) + "\n")


def _write_source_manifest(work_dir: Path, source_dir: Path) -> None:
    dependencies: dict[str, str] = {}
    for package in ("synapse-sdk", "synapse-core"):
        package_json = json.loads(
            (source_dir / "packages" / package / "package.json").read_text()
        )
        for name, version in package_json.get("peerDependencies", {}).items():
            existing = dependencies.get(name)
            if existing is not None and existing != version:
                raise RuntimeError(
                    f"Synapse source packages disagree on {name}: {existing} != {version}"
                )
            dependencies[name] = version
    manifest = {
        "name": "foc-devnet-synapse-source-e2e",
        "private": True,
        "type": "module",
        "dependencies": dependencies,
    }
    (work_dir / "package.json").write_text(json.dumps(manifest, indent=2) + "\n")
    policy = source_dir / "pnpm-workspace.yaml"
    if not policy.is_file():
        raise RuntimeError(f"Synapse source has no pnpm workspace policy: {policy}")
    shutil.copyfile(policy, work_dir / policy.name)


def _copy_scenarios(work_dir: Path) -> None:
    source = _scenario_dir()
    if not source.is_dir():
        raise RuntimeError(f"Synapse scenario directory not found: {source}")
    shutil.copytree(source, work_dir, dirs_exist_ok=True)


def _source_pnpm_version(source_dir: Path) -> str:
    package_manager = json.loads((source_dir / "package.json").read_text()).get(
        "packageManager"
    )
    if not isinstance(package_manager, str) or not package_manager.startswith("pnpm@"):
        raise RuntimeError(f"Synapse source has no declared pnpm version: {source_dir}")
    return package_manager.removeprefix("pnpm@")


def _has_source_package_closure(source_dir: Path) -> bool:
    node_modules = source_dir / "packages" / "synapse-sdk" / "node_modules"
    return (node_modules / "@filoz" / "synapse-core").is_dir()


def _source_commit(source_dir: Path) -> str:
    result = subprocess.run(
        ["git", "-C", str(source_dir), "rev-parse", "HEAD"],
        text=True,
        capture_output=True,
    )
    if result.returncode:
        raise RuntimeError(f"Cannot determine Synapse source commit at {source_dir}")
    return result.stdout.strip()


def _load_runtime(work_dir: Path) -> SynapseRuntime:
    marker = _runtime_marker(work_dir)
    if not marker.is_file():
        raise RuntimeError(f"SYNAPSE_RUNTIME_DIR is not a prepared runtime: {work_dir}")
    data = json.loads(marker.read_text())
    source_dir = data.get("source_dir")
    return SynapseRuntime(
        work_dir=work_dir,
        source=data["source"],
        provenance=data["provenance"],
        source_dir=Path(source_dir) if source_dir else None,
    )


def prepare_synapse_runtime(work_dir: Path) -> SynapseRuntime:
    """Prepare one reusable runtime and return its executable scenario location."""
    reused_dir = os.environ.get("SYNAPSE_RUNTIME_DIR")
    if reused_dir:
        return _load_runtime(Path(reused_dir).resolve())

    work_dir = work_dir.resolve()
    work_dir.mkdir(parents=True, exist_ok=True)
    dependency = component("synapse-sdk")
    local_source = os.environ.get("SYNAPSE_SDK_SOURCE_DIR")

    if local_source or dependency.get("source") != "npm":
        source_dir = (
            Path(local_source).resolve() if local_source else work_dir / "synapse-sdk"
        )
        if local_source:
            if not source_dir.is_dir():
                raise RuntimeError(
                    f"SYNAPSE_SDK_SOURCE_DIR is not a directory: {source_dir}"
                )
            provenance = f"local:{source_dir}@{_source_commit(source_dir)}"
        else:
            checkout = dependency.get("commit") or dependency.get("ref")
            if not checkout:
                raise RuntimeError("resolved Synapse source has no commit or ref")
            if not run_cmd(
                ["git", "clone", dependency["repository"], str(source_dir)],
                label="clone synapse-sdk",
            ):
                raise RuntimeError("failed to clone synapse-sdk")
            if not run_cmd(
                ["git", "checkout", "--detach", checkout],
                cwd=str(source_dir),
                label=f"checkout synapse-sdk {checkout}",
            ):
                raise RuntimeError("failed to checkout synapse-sdk")
            actual_commit = _source_commit(source_dir)
            expected_commit = dependency.get("commit")
            if expected_commit and actual_commit != expected_commit:
                raise RuntimeError(
                    f"synapse-sdk checkout is {actual_commit}, expected {expected_commit}"
                )
            provenance = f"git:{dependency['repository']}@{actual_commit}"

        pnpm_version = _source_pnpm_version(source_dir)
        source_node_modules = source_dir / "packages" / "synapse-sdk" / "node_modules"
        has_package_closure = _has_source_package_closure(source_dir)
        if not local_source or not has_package_closure:
            if not run_cmd(
                [
                    "pnpm",
                    "install",
                    "--no-frozen-lockfile",
                    "--prod",
                    "--ignore-scripts",
                    "--filter",
                    "@filoz/synapse-sdk...",
                ],
                cwd=str(source_dir),
                label=f"install Synapse production dependencies (pnpm@{pnpm_version})",
            ):
                raise RuntimeError("failed to install Synapse production dependencies")
        if not _has_source_package_closure(source_dir):
            raise RuntimeError(
                f"Synapse source install has no package closure: {source_node_modules}"
            )
        _write_source_manifest(work_dir, source_dir)
        if not run_cmd(
            [
                "pnpm",
                "install",
                "--no-frozen-lockfile",
                "--prod",
                "--ignore-scripts",
            ],
            cwd=str(work_dir),
            label="install Synapse peer runtime",
        ):
            raise RuntimeError("failed to install Synapse peer runtime")
        if not (work_dir / "node_modules" / "viem").is_dir():
            raise RuntimeError("Synapse peer runtime has no viem installation")
        provenance = f"{provenance} (pnpm@{pnpm_version})"
        runtime = SynapseRuntime(work_dir, "source", provenance, source_dir)
    else:
        _write_manifest(work_dir, dependency)
        if not run_cmd(
            [
                "npm",
                "install",
                "--omit=dev",
                "--ignore-scripts",
                "--package-lock=false",
            ],
            cwd=str(work_dir),
            label="install Synapse consumer runtime",
        ):
            raise RuntimeError("failed to install Synapse consumer runtime")
        runtime = SynapseRuntime(
            work_dir,
            "npm",
            f"npm:{dependency['package']}@{dependency['version']}",
        )

    _copy_scenarios(work_dir)
    _runtime_marker(work_dir).write_text(
        json.dumps(
            {
                "source": runtime.source,
                "provenance": runtime.provenance,
                "source_dir": str(runtime.source_dir) if runtime.source_dir else None,
            },
            indent=2,
        )
        + "\n"
    )
    info(f"Synapse runtime: {runtime.provenance}")
    return runtime


def run_node_script(
    runtime: SynapseRuntime,
    script_name: str,
    label: str,
    args: list[str] | None = None,
    env: dict | None = None,
    timeout: int | None = None,
) -> None:
    """Run a prepared scenario entrypoint with retries for transient state forks."""
    script = runtime.work_dir / script_name
    if not script.is_file():
        raise RuntimeError(f"Synapse scenario entrypoint not found: {script}")

    cmd = ["node"]
    process_env = {**os.environ, **(env or {})}
    if runtime.source_dir:
        loader = runtime.work_dir / "source-runtime.mjs"
        if not loader.is_file():
            raise RuntimeError(f"Synapse source runtime hook not found: {loader}")
        cmd.extend(["--import", str(loader)])
        process_env["SYNAPSE_SDK_SOURCE_DIR"] = str(runtime.source_dir)
    cmd.extend([str(script), *(args or [])])

    max_attempts = len(UPLOAD_RETRY_DELAYS_SECS) + 1
    for attempt in range(1, max_attempts + 1):
        result = subprocess.run(
            cmd,
            cwd=str(runtime.work_dir),
            env=process_env,
            text=True,
            capture_output=True,
            timeout=timeout,
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
