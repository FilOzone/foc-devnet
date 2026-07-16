import importlib.util
import json
import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from unittest.mock import patch

SCRIPT = Path(__file__).parents[1] / "resolve-ci-dependencies.py"
SPEC = importlib.util.spec_from_file_location("dependency_resolver", SCRIPT)
resolver = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(resolver)


class FakeRunner:
    def __init__(self, responses):
        self.responses = (
            list(responses.items()) if isinstance(responses, dict) else responses
        )

    def __call__(self, command):
        key = tuple(command)
        for matcher, response in self.responses:
            if callable(matcher):
                if matcher(command):
                    return response
            elif key == matcher:
                return response
        raise AssertionError(f"Unexpected command: {command}")


class ResolverTests(unittest.TestCase):
    def manifest(self, profiles=None, components=None):
        if components is None:
            components = {
                "lotus": self.component("a", "b"),
                "curio": self.component("c", "d"),
                "filecoin-services": self.component("e", "f"),
                "synapse-sdk": self.component("1", "2"),
                "filecoin-pin": self.component("3", "4"),
            }
        if profiles is None:
            profiles = {
                "default": {"base": "default"},
                "stability": {"base": "stability"},
                "frontier": {"base": "frontier"},
                "stability-frontier-lotus": {
                    "base": "stability",
                    "components": {"lotus": "frontier"},
                },
                "stability-frontier-curio": {
                    "base": "stability",
                    "components": {"curio": "frontier"},
                },
                "stability-frontier-filecoin-services": {
                    "base": "stability",
                    "components": {"filecoin-services": "frontier"},
                },
            }
        return {
            "schema_version": 2,
            "profiles": profiles,
            "components": components,
        }

    def component(self, stability_prefix, frontier_prefix):
        return {
            "repository": "https://example.test/project.git",
            "default": {"strategy": "config_default"},
            "stability": {
                "strategy": "git_commit",
                "commit": stability_prefix * 40,
            },
            "frontier": {
                "strategy": "git_commit",
                "commit": frontier_prefix * 40,
            },
        }

    def resolve_manifest(self, manifest, profile):
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            manifest_path = directory / "manifest.json"
            output_path = directory / "resolved.json"
            manifest_path.write_text(json.dumps(manifest))
            args = type(
                "Args",
                (),
                {
                    "profile": profile,
                    "manifest": manifest_path,
                    "output": output_path,
                    "github_output": None,
                    "github_env": None,
                },
            )
            with redirect_stdout(StringIO()):
                resolver.resolve(args)
            return json.loads(output_path.read_text())

    def test_latest_non_prerelease_tag_excludes_prereleases_and_annotated_refs(self):
        output = "\n".join(
            [
                "aaa refs/tags/v1.9.0",
                "bbb refs/tags/v2.0.0-rc1",
                "ccc refs/tags/v1.10.0",
                "ddd refs/tags/v1.10.0^{}",
                "eee refs/tags/not-a-version",
            ]
        )
        self.assertEqual(
            resolver.select_latest_non_prerelease_tag(output, "v*"),
            ("v1.10.0", "ddd"),
        )

    def test_prefixed_non_prerelease_tag(self):
        output = "\n".join(
            [
                "aaa refs/tags/pdp/v1.2.0",
                "bbb refs/tags/pdp/v1.11.0",
                "ccc refs/tags/v9.0.0",
            ]
        )
        self.assertEqual(
            resolver.select_latest_non_prerelease_tag(output, "pdp/v*"),
            ("pdp/v1.11.0", "bbb"),
        )

    def test_git_tag_pattern_can_include_prereleases(self):
        component = {
            "repository": "https://example.test/project.git",
            "stability": {
                "strategy": "git_tag",
                "tag": "v*",
                "include_prereleases": True,
            },
        }
        runner = FakeRunner(
            {
                (
                    "git",
                    "ls-remote",
                    "--tags",
                    "https://example.test/project.git",
                    "v*",
                ): "\n".join(
                    [
                        "aaa refs/tags/v1.10.0",
                        "bbb refs/tags/v2.0.0-rc1",
                        "ccc refs/tags/not-a-version",
                    ]
                ),
            }
        )

        resolved = resolver.resolve_component("project", component, "stability", runner)

        self.assertEqual(resolved["ref"], "v2.0.0-rc1")
        self.assertEqual(resolved["commit"], "bbb")

    def test_unknown_profile_fails(self):
        manifest = self.manifest()
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            manifest_path = directory / "manifest.json"
            output_path = directory / "resolved.json"
            manifest_path.write_text(json.dumps(manifest))
            args = type(
                "Args",
                (),
                {
                    "profile": "unknown",
                    "manifest": manifest_path,
                    "output": output_path,
                    "github_output": None,
                    "github_env": None,
                },
            )
            with self.assertRaisesRegex(resolver.ResolutionError, "Unknown profile"):
                resolver.resolve(args)

    def test_manifest_missing_component_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "manifest.json"
            path.write_text(
                json.dumps(
                    {
                        "schema_version": 2,
                        "profiles": {"default": {"base": "default"}},
                        "components": {},
                    }
                )
            )
            with self.assertRaisesRegex(resolver.ResolutionError, "missing"):
                resolver.load_manifest(path)

    def test_mixed_profile_resolves_only_selected_component_from_frontier(self):
        metadata = self.resolve_manifest(self.manifest(), "stability-frontier-curio")
        components = metadata["components"]

        self.assertEqual(metadata["profile"], "stability-frontier-curio")
        self.assertEqual(components["lotus"]["selection_profile"], "stability")
        self.assertEqual(components["lotus"]["commit"], "a" * 40)
        self.assertEqual(components["curio"]["selection_profile"], "frontier")
        self.assertEqual(components["curio"]["commit"], "d" * 40)
        self.assertEqual(
            components["filecoin-services"]["selection_profile"], "stability"
        )
        self.assertEqual(components["synapse-sdk"]["selection_profile"], "stability")
        self.assertEqual(components["filecoin-pin"]["selection_profile"], "stability")

    def test_absent_mixed_profile_is_rejected(self):
        profiles = {
            "default": {"base": "default"},
            "stability": {"base": "stability"},
            "frontier": {"base": "frontier"},
            "stability-frontier-curio": {
                "base": "stability",
                "components": {"curio": "frontier"},
            },
        }
        manifest = self.manifest(profiles=profiles)
        with self.assertRaisesRegex(resolver.ResolutionError, "Unknown profile"):
            self.resolve_manifest(manifest, "stability-frontier-filecoin-pin")

    def test_manifest_profile_rejects_unknown_component_override(self):
        manifest = self.manifest(
            profiles={
                "default": {"base": "default"},
                "bad": {
                    "base": "stability",
                    "components": {"missing": "frontier"},
                },
            }
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "manifest.json"
            path.write_text(json.dumps(manifest))
            with self.assertRaisesRegex(resolver.ResolutionError, "unknown components"):
                resolver.load_manifest(path)

    def test_manifest_profile_rejects_missing_component_selection(self):
        manifest = self.manifest(
            profiles={
                "default": {"base": "default"},
                "bad": {
                    "base": "stability",
                    "components": {"lotus": "not-a-selection"},
                },
            }
        )
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "manifest.json"
            path.write_text(json.dumps(manifest))
            with self.assertRaisesRegex(resolver.ResolutionError, "no such selection"):
                resolver.load_manifest(path)

    def test_npm_version_resolves_dist_tag_to_npm_version(self):
        component = {
            "repository": "https://example.test/filecoin-pin.git",
            "npm_package": "filecoin-pin",
            "stability": {"strategy": "npm_version", "version": "latest"},
        }
        runner = FakeRunner(
            {
                (
                    "npm",
                    "view",
                    "filecoin-pin@latest",
                    "version",
                    "--json",
                ): '"1.1.1"',
                (
                    "npm",
                    "view",
                    "filecoin-pin@1.1.1",
                    "gitHead",
                    "--json",
                ): '""',
            }
        )

        resolved = resolver.resolve_component(
            "filecoin-pin", component, "stability", runner
        )

        self.assertEqual(resolved["source"], "npm")
        self.assertEqual(resolved["version"], "1.1.1")

    def test_profile_overrides_are_copied_to_resolved_component(self):
        component = {
            "repository": "https://example.test/filecoin-pin.git",
            "npm_package": "filecoin-pin",
            "default": {
                "strategy": "npm_version",
                "version": "1.0.1",
                "overrides": {"nanoid": {"version": "3.3.13", "reason": "test"}},
            },
        }
        runner = FakeRunner(
            {
                (
                    "npm",
                    "view",
                    "filecoin-pin@1.0.1",
                    "version",
                    "--json",
                ): '"1.0.1"',
                (
                    "npm",
                    "view",
                    "filecoin-pin@1.0.1",
                    "gitHead",
                    "--json",
                ): '""',
            }
        )

        resolved = resolver.resolve_component(
            "filecoin-pin", component, "default", runner
        )

        self.assertEqual(
            resolved["overrides"],
            {"nanoid": {"version": "3.3.13", "reason": "test"}},
        )

    def test_profile_overrides_require_version_and_reason(self):
        component = {
            "repository": "https://example.test/filecoin-pin.git",
            "npm_package": "filecoin-pin",
            "default": {
                "strategy": "npm_version",
                "version": "1.0.1",
                "overrides": {"nanoid": {"version": "3.3.13"}},
            },
        }
        runner = FakeRunner(
            {
                (
                    "npm",
                    "view",
                    "filecoin-pin@1.0.1",
                    "version",
                    "--json",
                ): '"1.0.1"',
                (
                    "npm",
                    "view",
                    "filecoin-pin@1.0.1",
                    "gitHead",
                    "--json",
                ): '""',
            }
        )

        with self.assertRaisesRegex(resolver.ResolutionError, "reason"):
            resolver.resolve_component("filecoin-pin", component, "default", runner)

    def test_overrides_are_rejected_for_core_git_components(self):
        component = {
            "repository": "https://example.test/lotus.git",
            "frontier": {
                "strategy": "git_branch",
                "branch": "master",
                "overrides": {"nanoid": {"version": "3.3.13", "reason": "test"}},
            },
        }
        runner = FakeRunner(
            {
                (
                    "git",
                    "ls-remote",
                    "https://example.test/lotus.git",
                    "refs/heads/master",
                ): "deadbeef refs/heads/master",
            }
        )

        with self.assertRaisesRegex(resolver.ResolutionError, "not supported"):
            resolver.resolve_component("lotus", component, "frontier", runner)

    def test_overrides_are_rejected_for_source_built_filecoin_pin(self):
        component = {
            "repository": "https://example.test/filecoin-pin.git",
            "frontier": {
                "strategy": "git_branch",
                "branch": "master",
                "overrides": {"nanoid": {"version": "3.3.13", "reason": "test"}},
            },
        }
        runner = FakeRunner(
            {
                (
                    "git",
                    "ls-remote",
                    "https://example.test/filecoin-pin.git",
                    "refs/heads/master",
                ): "deadbeef refs/heads/master",
            }
        )

        with self.assertRaisesRegex(resolver.ResolutionError, "not supported"):
            resolver.resolve_component("filecoin-pin", component, "frontier", runner)

    def test_frontier_branch_resolves_to_commit(self):
        component = {
            "repository": "https://example.test/project.git",
            "frontier": {"strategy": "git_branch", "branch": "main"},
        }
        runner = FakeRunner(
            {
                (
                    "git",
                    "ls-remote",
                    "https://example.test/project.git",
                    "refs/heads/main",
                ): "deadbeef refs/heads/main",
            }
        )
        resolved = resolver.resolve_component("project", component, "frontier", runner)
        self.assertEqual(resolved["commit"], "deadbeef")

    def test_git_commit_strategy_uses_exact_sha_without_resolution(self):
        commit = "fadc836e65804311aca3bd2276861acabe42313f"
        component = {
            "repository": "https://example.test/synapse.git",
            "default": {"strategy": "git_commit", "commit": commit},
        }
        resolved = resolver.resolve_component(
            "synapse-sdk", component, "default", FakeRunner({})
        )
        self.assertEqual(resolved["ref_type"], "commit")
        self.assertEqual(resolved["ref"], commit)
        self.assertEqual(resolved["commit"], commit)

    def test_git_commit_strategy_rejects_non_sha(self):
        component = {
            "repository": "https://example.test/synapse.git",
            "default": {"strategy": "git_commit", "commit": "master"},
        }
        with self.assertRaisesRegex(resolver.ResolutionError, "invalid commit SHA"):
            resolver.resolve_component(
                "synapse-sdk", component, "default", FakeRunner({})
            )

    def test_init_args_skip_config_defaults_and_pin_other_sources(self):
        components = {
            "lotus": {"source": "config_default"},
            "curio": {
                "source": "git",
                "repository": "https://example.test/curio.git",
                "commit": "abc",
            },
            "filecoin-services": {
                "source": "git",
                "repository": "https://example.test/services.git",
                "commit": "def",
            },
        }
        self.assertEqual(
            resolver.build_init_args(components),
            [
                "--curio",
                "gitcommit:https://example.test/curio.git:abc",
                "--filecoin-services",
                "gitcommit:https://example.test/services.git:def",
            ],
        )

    def test_cache_hash_depends_only_on_lotus_and_curio_commits(self):
        base = {
            "lotus": {"commit": "aaa"},
            "curio": {"commit": "bbb"},
            "filecoin-services": {"commit": "ccc"},
        }
        first = resolver.cache_hash(base)
        base["filecoin-services"]["commit"] = "changed"
        self.assertEqual(first, resolver.cache_hash(base))
        base["curio"]["commit"] = "changed"
        self.assertNotEqual(first, resolver.cache_hash(base))

    @patch.object(resolver, "run_command")
    def test_verify_records_checkouts_and_writes_cache_key(self, run_command):
        commits = {
            "lotus": "aaa",
            "curio": "bbb",
            "filecoin-services": "ccc",
        }
        run_command.side_effect = lambda command: commits[Path(command[2]).name]
        metadata = {
            "schema_version": 1,
            "profile": "default",
            "components": {name: {"source": "config_default"} for name in commits},
        }
        with tempfile.TemporaryDirectory() as directory:
            directory = Path(directory)
            metadata_path = directory / "metadata.json"
            output_path = directory / "output"
            metadata_path.write_text(json.dumps(metadata))
            args = type(
                "Args",
                (),
                {
                    "metadata": metadata_path,
                    "code_dir": directory / "code",
                    "github_output": str(output_path),
                },
            )
            resolver.verify(args)
            verified = json.loads(metadata_path.read_text())
            outputs = output_path.read_text()

        self.assertEqual(verified["components"]["lotus"]["commit"], "aaa")
        self.assertTrue(verified["components"]["curio"]["verified"])
        self.assertIn("source-hash=", outputs)


if __name__ == "__main__":
    unittest.main()
