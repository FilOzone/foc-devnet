#!/usr/bin/env python3
"""Access resolved dependency metadata from scenario tests."""

from __future__ import annotations

import json
import os
import tomllib
from pathlib import Path


def metadata_path() -> Path | None:
    value = os.environ.get("CI_DEPENDENCY_METADATA")
    return Path(value) if value else None


def load_metadata() -> dict:
    path = metadata_path()
    if path is None or not path.is_file():
        return {}
    with path.open() as file:
        return json.load(file)


def component(name: str) -> dict:
    components = load_metadata().get("components", {})
    value = components.get(name)
    if isinstance(value, dict):
        return value

    manifest_path = Path(__file__).parents[1] / "dependencies.toml"
    manifest = tomllib.loads(manifest_path.read_text())
    definition = manifest["dependencies"].get(name)
    if not definition:
        raise RuntimeError(f"Dependency manifest has no {name!r} dependency")
    selection = definition["default"]
    fallback = {
        "name": name,
        "repository": definition["repository"],
        "strategy": selection["strategy"],
    }
    if "overrides" in selection:
        fallback["overrides"] = selection["overrides"]
    if selection["strategy"] == "git_commit":
        fallback.update(
            source="git",
            ref=selection["commit"],
            commit=selection["commit"],
        )
    elif selection["strategy"] == "git_tag":
        fallback.update(source="git", ref=selection["tag"])
    elif selection["strategy"] == "npm_version":
        fallback.update(
            source="npm",
            package=definition["npm_package"],
            version=selection["version"],
        )
    else:
        raise RuntimeError(
            f"Local scenario fallback does not support {selection['strategy']!r}"
        )
    return fallback


def format_markdown_table(metadata: dict | None = None) -> str:
    if metadata is None:
        metadata = load_metadata()
    components = metadata.get("components", {})
    if not components:
        return "Resolved dependency metadata is unavailable."

    rows = [
        "| Dependency | Source | Version/ref | Commit | Overrides |",
        "|---|---|---|---|---|",
    ]
    for name, value in components.items():
        source = value.get("source", value.get("strategy", "-"))
        version = value.get("version") or value.get("ref") or "-"
        commit = value.get("commit") or "-"
        overrides = value.get("overrides") or {}
        overrides_text = (
            ", ".join(
                f"`{package}={spec['version']}`"
                for package, spec in sorted(overrides.items())
            )
            or "-"
        )
        rows.append(
            f"| {name} | {source} | `{version}` | `{commit}` | {overrides_text} |"
        )
    return "\n".join(rows)
