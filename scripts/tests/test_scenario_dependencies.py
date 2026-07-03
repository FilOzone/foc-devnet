import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from scenarios.dependencies import format_markdown_table
from scenarios.synapse import clone_and_build
from scenarios.test_multi_copy_upload import setup_filecoin_pin


class ScenarioDependencyTests(unittest.TestCase):
    def test_dependency_table_contains_all_resolved_components(self):
        metadata = {
            "components": {
                "lotus": {
                    "source": "git",
                    "ref": "v1.2.3",
                    "commit": "aaa",
                },
                "synapse-sdk": {
                    "source": "git",
                    "version": "0.41.0",
                    "commit": "bbb",
                    "overrides": {"nanoid": {"version": "3.3.13", "reason": "test"}},
                },
                "filecoin-pin": {
                    "source": "npm",
                    "version": "1.0.1",
                    "commit": "ccc",
                },
            }
        }
        table = format_markdown_table(metadata)
        for expected in (
            "lotus",
            "synapse-sdk",
            "filecoin-pin",
            "nanoid",
            "aaa",
            "1.0.1",
            "3.3.13",
        ):
            self.assertIn(expected, table)

    @patch("scenarios.synapse.sh", return_value="deadbeef")
    @patch("scenarios.synapse.run_cmd", return_value=True)
    @patch(
        "scenarios.synapse.component",
        return_value={
            "repository": "https://example.test/synapse.git",
            "ref": "sdk-v1.0.0",
            "commit": "deadbeef",
            "overrides": {"nanoid": {"version": "3.3.13", "reason": "test"}},
        },
    )
    def test_synapse_checkout_uses_resolved_commit(self, _component, run_cmd, _sh):
        def fake_run_cmd(command, **_kwargs):
            if command[:2] == ["git", "clone"]:
                sdk_dir = Path(command[3])
                sdk_dir.mkdir(parents=True)
                (sdk_dir / "pnpm-workspace.yaml").write_text(
                    "packages:\n  - packages/*\n"
                )
            return True

        run_cmd.side_effect = fake_run_cmd
        with tempfile.TemporaryDirectory() as directory:
            clone_and_build(Path(directory))
            workspace = Path(directory) / "synapse-sdk" / "pnpm-workspace.yaml"
            workspace_text = workspace.read_text()
        checkout = run_cmd.call_args_list[1]
        self.assertEqual(
            checkout.args[0],
            ["git", "checkout", "--detach", "deadbeef"],
        )
        commands = [call.args[0] for call in run_cmd.call_args_list]
        self.assertNotIn(["pnpm", "pkg", "set"], [command[:3] for command in commands])
        self.assertIn('  "nanoid": "3.3.13"', workspace_text)

    @patch("scenarios.test_multi_copy_upload.run_cmd", return_value=True)
    @patch(
        "scenarios.test_multi_copy_upload.component",
        return_value={
            "source": "npm",
            "version": "1.0.1",
            "overrides": {"nanoid": {"version": "3.3.13", "reason": "test"}},
        },
    )
    def test_filecoin_pin_npm_install_path(self, _component, run_cmd):
        with tempfile.TemporaryDirectory() as directory:
            command, dependency_dir = setup_filecoin_pin(Path(directory))
        self.assertTrue(command[0].endswith("node_modules/.bin/filecoin-pin"))
        self.assertEqual(dependency_dir, Path(directory))
        self.assertIn(
            "dependencies.filecoin-pin=1.0.1",
            run_cmd.call_args_list[1].args[0],
        )
        self.assertIn(
            "dependencies.nanoid=3.3.13",
            run_cmd.call_args_list[1].args[0],
        )
        self.assertIn(
            "overrides.nanoid=3.3.13",
            run_cmd.call_args_list[1].args[0],
        )

    @patch("scenarios.test_multi_copy_upload.run_cmd", return_value=True)
    @patch(
        "scenarios.test_multi_copy_upload.component",
        return_value={
            "source": "git",
            "repository": "https://example.test/filecoin-pin.git",
            "commit": "deadbeef",
        },
    )
    def test_filecoin_pin_frontier_build_path(self, _component, run_cmd):
        with tempfile.TemporaryDirectory() as directory:
            command, dependency_dir = setup_filecoin_pin(Path(directory))
        self.assertEqual(command[0], "node")
        self.assertTrue(command[1].endswith("filecoin-pin/dist/cli.js"))
        self.assertTrue(str(dependency_dir).endswith("filecoin-pin"))
        commands = [call.args[0] for call in run_cmd.call_args_list]
        self.assertIn(["git", "checkout", "--detach", "deadbeef"], commands)
        self.assertNotIn(["pnpm", "pkg", "set"], [command[:3] for command in commands])
        self.assertIn(
            ["pnpm", "install", "--frozen-lockfile", "--filter", "filecoin-pin..."],
            commands,
        )
        self.assertIn(["pnpm", "build"], commands)


if __name__ == "__main__":
    unittest.main()
