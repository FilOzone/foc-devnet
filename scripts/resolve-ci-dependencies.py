#!/usr/bin/env python3
"""Resolve CI dependency profiles to immutable versions and verify checkouts."""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import json
import os
import re
import shlex
import subprocess
import tempfile
from pathlib import Path

MANIFEST_SCHEMA_VERSION = 2
INIT_COMPONENT_FLAGS = {
    "lotus": "--lotus",
    "curio": "--curio",
    "filecoin-services": "--filecoin-services",
    "pdp": "--pdp",
}
VERSION_RE = re.compile(r"^\d+(?:\.\d+)*(?:[-+][0-9A-Za-z.-]+)?$")
STABLE_VERSION_RE = re.compile(r"^\d+(?:\.\d+)*$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")


class ResolutionError(RuntimeError):
    pass


def run_command(command: list[str]) -> str:
    result = subprocess.run(command, text=True, capture_output=True)
    if result.returncode != 0:
        details = (result.stderr or result.stdout).strip()
        raise ResolutionError(f"{shlex.join(command)} failed: {details}")
    return result.stdout.strip()


def load_manifest(path: Path) -> dict:
    try:
        manifest = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise ResolutionError(
            f"Cannot load dependency manifest {path}: {error}"
        ) from error

    if manifest.get("schema_version") != MANIFEST_SCHEMA_VERSION:
        raise ResolutionError(
            f"Dependency manifest schema_version must be {MANIFEST_SCHEMA_VERSION}"
        )
    components = manifest.get("components")
    if not isinstance(components, dict):
        raise ResolutionError("Dependency manifest must contain a components object")
    profiles = manifest.get("profiles")
    if not isinstance(profiles, dict):
        raise ResolutionError("Dependency manifest must contain a profiles object")

    required = set(INIT_COMPONENT_FLAGS) | {"synapse-sdk", "filecoin-pin"}
    missing = sorted(required - set(components))
    if missing:
        raise ResolutionError(f"Dependency manifest is missing: {', '.join(missing)}")
    validate_profiles(profiles, components)
    return manifest


def validate_profiles(profiles: dict, components: dict) -> None:
    for profile_name, profile in profiles.items():
        if not isinstance(profile_name, str) or not profile_name:
            raise ResolutionError("Profile names must be non-empty strings")
        if not isinstance(profile, dict):
            raise ResolutionError(f"Profile {profile_name!r} must be an object")

        base = profile.get("base")
        if not isinstance(base, str) or not base:
            raise ResolutionError(f"Profile {profile_name!r} base must be a string")

        component_overrides = profile.get("components", {})
        if not isinstance(component_overrides, dict):
            raise ResolutionError(
                f"Profile {profile_name!r} components must be an object"
            )

        unknown_components = sorted(set(component_overrides) - set(components))
        if unknown_components:
            raise ResolutionError(
                f"Profile {profile_name!r} references unknown components: "
                f"{', '.join(unknown_components)}"
            )

        for component_name, component in components.items():
            selection_profile = component_overrides.get(component_name, base)
            if not isinstance(selection_profile, str) or not selection_profile:
                raise ResolutionError(
                    f"Profile {profile_name!r} selection for {component_name!r} "
                    "must be a string"
                )
            if selection_profile not in component:
                raise ResolutionError(
                    f"Profile {profile_name!r} selects {selection_profile!r} for "
                    f"{component_name}, but that component has no such selection"
                )


def component_profile_map(manifest: dict, profile_name: str) -> dict[str, str]:
    profiles = manifest["profiles"]
    if profile_name not in profiles:
        raise ResolutionError(f"Unknown profile {profile_name!r}")

    profile = profiles[profile_name]
    base = profile["base"]
    component_overrides = profile.get("components", {})
    return {
        component_name: component_overrides.get(component_name, base)
        for component_name in manifest["components"]
    }


def parse_ls_remote(output: str) -> list[tuple[str, str]]:
    refs = []
    for line in output.splitlines():
        fields = line.split()
        if len(fields) == 2:
            refs.append((fields[0], fields[1]))
    return refs


def resolve_ref(repository: str, ref: str, runner=run_command) -> str:
    refs = parse_ls_remote(runner(["git", "ls-remote", repository, ref]))
    if not refs:
        raise ResolutionError(f"No ref {ref} found in {repository}")
    return refs[0][0]


def resolve_tag(repository: str, tag: str, runner=run_command) -> str:
    refs = parse_ls_remote(
        runner(["git", "ls-remote", repository, f"refs/tags/{tag}*"])
    )
    target = f"refs/tags/{tag}"
    exact_refs = [
        (commit, ref) for commit, ref in refs if ref in {target, f"{target}^{{}}"}
    ]
    if not exact_refs:
        raise ResolutionError(f"No tag {tag} found in {repository}")
    peeled = next((commit for commit, ref in exact_refs if ref.endswith("^{}")), None)
    return peeled or exact_refs[0][0]


def tag_version(tag: str, pattern: str) -> str | None:
    if not fnmatch.fnmatchcase(tag, pattern):
        return None
    star = pattern.find("*")
    version = tag
    if star >= 0:
        prefix = pattern[:star]
        suffix = pattern[star + 1 :]
        version = tag[len(prefix) :]
        if suffix:
            version = version[: -len(suffix)]
    version = version.removeprefix("v")
    if not VERSION_RE.fullmatch(version):
        return None
    return version


def stable_version(tag: str, pattern: str) -> tuple[int, ...] | None:
    version = tag_version(tag, pattern)
    if version is None:
        return None
    if not STABLE_VERSION_RE.fullmatch(version):
        return None
    return tuple(int(part) for part in version.split("."))


def version_sort_key(version: str) -> tuple[tuple[int, ...], int, str]:
    public_version = version.split("+", 1)[0]
    numeric_text, separator, prerelease = public_version.partition("-")
    numeric = tuple(int(part) for part in numeric_text.split("."))
    return numeric, 0 if separator else 1, prerelease


def select_latest_non_prerelease_tag(output: str, pattern: str) -> tuple[str, str]:
    tag_commits = {}
    peeled_commits = {}
    for commit, ref in parse_ls_remote(output):
        if not ref.startswith("refs/tags/"):
            continue
        tag = ref.removeprefix("refs/tags/").removesuffix("^{}")
        if ref.endswith("^{}"):
            peeled_commits[tag] = commit
        else:
            tag_commits[tag] = commit

    candidates = []
    for tag, commit in tag_commits.items():
        version = stable_version(tag, pattern)
        if version is not None:
            candidates.append((version, tag, peeled_commits.get(tag, commit)))
    if not candidates:
        raise ResolutionError(f"No stable tags match {pattern}")
    _, tag, commit = max(candidates)
    return tag, commit


def select_latest_tag(
    output: str, pattern: str, include_prereleases: bool = False
) -> tuple[str, str]:
    if not include_prereleases:
        return select_latest_non_prerelease_tag(output, pattern)

    tag_commits = {}
    peeled_commits = {}
    for commit, ref in parse_ls_remote(output):
        if not ref.startswith("refs/tags/"):
            continue
        tag = ref.removeprefix("refs/tags/").removesuffix("^{}")
        if ref.endswith("^{}"):
            peeled_commits[tag] = commit
        else:
            tag_commits[tag] = commit

    candidates = []
    for tag, commit in tag_commits.items():
        version = tag_version(tag, pattern)
        if version is not None:
            candidates.append(
                (version_sort_key(version), tag, peeled_commits.get(tag, commit))
            )
    if not candidates:
        raise ResolutionError(f"No tags match {pattern}")
    _, tag, commit = max(candidates)
    return tag, commit


def npm_version(value, package: str, requested: str) -> str:
    """Normalize `npm view ... version --json` output.

    npm returns one string for an exact version or dist-tag and an ordered list
    for a range; the final non-empty list entry is the newest match.
    """
    if isinstance(value, str) and value:
        return value
    if isinstance(value, list):
        for version in reversed(value):
            if isinstance(version, str) and version:
                return version
    raise ResolutionError(f"npm returned no version for {package}@{requested}")


def npm_metadata(package: str, version: str, runner=run_command) -> dict:
    resolved_version = npm_version(
        json.loads(
            runner(["npm", "view", f"{package}@{version}", "version", "--json"])
        ),
        package,
        version,
    )
    git_head_output = runner(
        ["npm", "view", f"{package}@{resolved_version}", "gitHead", "--json"]
    )
    git_head = json.loads(git_head_output) if git_head_output else ""
    return {"version": resolved_version, "gitHead": git_head}


def npm_runtime_dependencies(
    package: str, version: str, runner=run_command
) -> dict[str, str]:
    metadata = json.loads(
        runner(
            [
                "npm",
                "view",
                f"{package}@{version}",
                "dependencies",
                "peerDependencies",
                "--json",
            ]
        )
    )
    dependencies = metadata.get("dependencies", {})
    peer_dependencies = metadata.get("peerDependencies", {})
    core_range = dependencies.get("@filoz/synapse-core")
    viem_range = peer_dependencies.get("viem") or dependencies.get("viem")
    if not isinstance(core_range, str) or not isinstance(viem_range, str):
        raise ResolutionError(
            f"{package}@{version} must declare @filoz/synapse-core and viem"
        )

    core = npm_metadata("@filoz/synapse-core", core_range, runner)["version"]
    viem = npm_metadata("viem", viem_range, runner)["version"]
    return {"@filoz/synapse-core": core, "viem": viem}


def read_gitlink(repository: str, commit: str, path: str, runner=run_command) -> str:
    with tempfile.TemporaryDirectory(prefix="foc-devnet-ci-deps-") as directory:
        repo_dir = Path(directory) / "repo"
        runner(["git", "-C", directory, "init", "repo"])
        runner(["git", "-C", str(repo_dir), "remote", "add", "origin", repository])
        runner(["git", "-C", str(repo_dir), "fetch", "--depth=1", "origin", commit])
        output = runner(["git", "-C", str(repo_dir), "ls-tree", "FETCH_HEAD", path])

    fields = output.split()
    if len(fields) < 4 or fields[0] != "160000" or fields[1] != "commit":
        raise ResolutionError(
            f"{path} in {repository}@{commit} is not a git submodule gitlink"
        )
    gitlink = fields[2]
    if not COMMIT_RE.fullmatch(gitlink):
        raise ResolutionError(
            f"{path} in {repository}@{commit} has invalid gitlink SHA {gitlink!r}"
        )
    return gitlink


def validate_overrides(name: str, strategy: str, overrides) -> dict:
    if overrides is None:
        return {}
    if not isinstance(overrides, dict):
        raise ResolutionError(f"{name} overrides must be a map")
    for package, spec in overrides.items():
        if (
            not isinstance(package, str)
            or not isinstance(spec, dict)
            or set(spec) != {"version", "reason"}
            or not all(isinstance(spec[key], str) and spec[key] for key in spec)
        ):
            raise ResolutionError(
                f"{name} override {package!r} must be an object with non-empty "
                "string 'version' and 'reason' fields"
            )

    if name in {"synapse-sdk", "filecoin-pin"} and strategy == "npm_version":
        return dict(sorted(overrides.items()))
    raise ResolutionError(
        f"{name} overrides are not supported with strategy {strategy!r}"
    )


def resolve_component(
    name: str, component: dict, profile: str, runner=run_command
) -> dict:
    try:
        selection = component[profile]
        strategy = selection["strategy"]
        repository = component["repository"]
    except KeyError as error:
        raise ResolutionError(f"{name} is missing required field {error}") from error

    resolved = {
        "name": name,
        "repository": repository,
        "selection_profile": profile,
        "strategy": strategy,
    }

    if strategy == "config_default":
        resolved["source"] = "config_default"
    elif strategy == "git_commit":
        commit = selection["commit"]
        if not COMMIT_RE.fullmatch(commit):
            raise ResolutionError(f"{name} has invalid commit SHA {commit!r}")
        resolved.update(source="git", ref_type="commit", ref=commit, commit=commit)
    elif strategy == "git_tag":
        tag = selection["tag"]
        if any(char in tag for char in "*?["):
            include_prereleases = selection.get("include_prereleases", False)
            if not isinstance(include_prereleases, bool):
                raise ResolutionError(f"{name} include_prereleases must be a boolean")
            output = runner(["git", "ls-remote", "--tags", repository, tag])
            tag, commit = select_latest_tag(output, tag, include_prereleases)
        else:
            commit = resolve_tag(repository, tag, runner)
        resolved.update(source="git", ref_type="tag", ref=tag, commit=commit)
    elif strategy == "git_branch":
        branch = selection["branch"]
        commit = resolve_ref(repository, f"refs/heads/{branch}", runner)
        resolved.update(source="git", ref_type="branch", ref=branch, commit=commit)
    elif strategy == "git_submodule":
        parent_repository = selection["repository"]
        tag = selection["tag"]
        path = selection["path"]
        if not isinstance(path, str) or not path:
            raise ResolutionError(f"{name} git_submodule path must be a string")
        if path.startswith("/") or ".." in Path(path).parts:
            raise ResolutionError(f"{name} git_submodule path must be relative")
        if any(char in tag for char in "*?["):
            include_prereleases = selection.get("include_prereleases", False)
            if not isinstance(include_prereleases, bool):
                raise ResolutionError(f"{name} include_prereleases must be a boolean")
            output = runner(["git", "ls-remote", "--tags", parent_repository, tag])
            tag, parent_commit = select_latest_tag(output, tag, include_prereleases)
        else:
            parent_commit = resolve_tag(parent_repository, tag, runner)
        commit = read_gitlink(parent_repository, parent_commit, path, runner)
        resolved.update(
            source="git_submodule",
            ref_type="commit",
            ref=commit,
            commit=commit,
            submodule_from={
                "repository": parent_repository,
                "ref_type": "tag",
                "ref": tag,
                "commit": parent_commit,
                "path": path,
            },
        )
    elif strategy == "npm_version":
        requested = selection["version"]
        data = npm_metadata(component["npm_package"], requested, runner)
        resolved.update(
            source="npm",
            package=component["npm_package"],
            version=data["version"],
            commit=data.get("gitHead", ""),
        )
        if name == "synapse-sdk":
            resolved["runtime_dependencies"] = npm_runtime_dependencies(
                component["npm_package"], data["version"], runner
            )
    else:
        raise ResolutionError(f"Unsupported strategy {strategy!r} for {name}")
    overrides = selection.get("overrides")
    if overrides is not None:
        resolved["overrides"] = validate_overrides(name, strategy, overrides)
    return resolved


def build_init_args(components: dict) -> list[str]:
    args = []
    for name, flag in INIT_COMPONENT_FLAGS.items():
        component = components[name]
        if component["source"] == "config_default":
            continue
        args.extend(
            [
                flag,
                f"gitcommit:{component['repository']}:{component['commit']}",
            ]
        )
    return args


def cache_hash(components: dict) -> str:
    values = [f"{name}:{components[name]['commit']}" for name in ("lotus", "curio")]
    return hashlib.sha256("\n".join(values).encode()).hexdigest()


def write_github_file(path: str | None, values: dict) -> None:
    if not path:
        return
    with open(path, "a") as output:
        for key, value in values.items():
            output.write(f"{key}={value}\n")


def scenario_environment(metadata_path: Path, components: dict) -> dict:
    synapse = components["synapse-sdk"]
    filecoin_pin = components["filecoin-pin"]
    return {
        "CI_DEPENDENCY_METADATA": str(metadata_path.resolve()),
        "SYNAPSE_SDK_SOURCE": synapse["source"],
        "SYNAPSE_SDK_REF": synapse.get("ref", ""),
        "SYNAPSE_SDK_COMMIT": synapse.get("commit", ""),
        "FILECOIN_PIN_SOURCE": filecoin_pin["source"],
        "FILECOIN_PIN_VERSION": filecoin_pin.get("version", ""),
        "FILECOIN_PIN_COMMIT": filecoin_pin.get("commit", ""),
        "PDP_SOURCE": components["pdp"]["source"],
        "PDP_REF": components["pdp"].get("ref", ""),
        "PDP_COMMIT": components["pdp"].get("commit", ""),
    }


def resolve(args) -> None:
    manifest = load_manifest(args.manifest)
    component_profiles = component_profile_map(manifest, args.profile)
    components = {
        name: resolve_component(name, component, component_profiles[name], run_command)
        for name, component in manifest["components"].items()
    }
    metadata = {
        "schema_version": 1,
        "profile": args.profile,
        "components": components,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(metadata, indent=2, sort_keys=True) + "\n")

    init_args = shlex.join(build_init_args(components))
    write_github_file(args.github_output, {"init-args": init_args})
    write_github_file(args.github_env, scenario_environment(args.output, components))
    print(json.dumps(metadata, indent=2, sort_keys=True))


def verify(args) -> None:
    metadata = json.loads(args.metadata.read_text())
    components = metadata["components"]
    for name in INIT_COMPONENT_FLAGS:
        if name == "pdp" and components[name]["source"] == "config_default":
            continue
        repository_path = args.code_dir / name
        actual = run_command(["git", "-C", str(repository_path), "rev-parse", "HEAD"])
        expected = components[name].get("commit")
        if expected and actual != expected:
            raise ResolutionError(f"{name} checkout is {actual}, expected {expected}")
        components[name]["commit"] = actual
        components[name]["verified"] = True

    args.metadata.write_text(json.dumps(metadata, indent=2, sort_keys=True) + "\n")
    source_hash = cache_hash(components)
    write_github_file(
        args.github_output,
        {"source-hash": source_hash, "go-cache-key": source_hash},
    )
    print(f"Verified dependency checkouts; source hash: {source_hash}")


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser()
    subparsers = root.add_subparsers(dest="operation", required=True)

    resolve_parser = subparsers.add_parser("resolve")
    resolve_parser.add_argument("--profile", required=True)
    resolve_parser.add_argument(
        "--manifest", type=Path, default=Path("ci/dependency-profiles.json")
    )
    resolve_parser.add_argument("--output", type=Path, required=True)
    resolve_parser.add_argument("--github-output")
    resolve_parser.add_argument("--github-env")
    resolve_parser.set_defaults(handler=resolve)

    verify_parser = subparsers.add_parser("verify")
    verify_parser.add_argument("--metadata", type=Path, required=True)
    verify_parser.add_argument(
        "--code-dir", type=Path, default=Path.home() / ".foc-devnet" / "code"
    )
    verify_parser.add_argument("--github-output")
    verify_parser.set_defaults(handler=verify)
    return root


def main() -> None:
    args = parser().parse_args()
    try:
        args.handler(args)
    except (ResolutionError, KeyError, json.JSONDecodeError) as error:
        raise SystemExit(f"dependency resolution failed: {error}") from error


if __name__ == "__main__":
    main()
