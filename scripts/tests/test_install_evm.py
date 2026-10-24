import os
from pathlib import Path
import subprocess
import tempfile
import unittest

INSTALLER = (
    Path(__file__).resolve().parents[2]
    / "src/commands/start/foc_deployer/install_evm.sh"
)


class InstallEvmTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="evm test ")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.contracts = self.root / "service_contracts"
        self.contracts.mkdir()
        self.home = self.root / "home"
        self.home.mkdir()
        self.temp = self.root / "temp"
        self.temp.mkdir()

    def run_installer(self):
        return subprocess.run(
            ["bash", str(INSTALLER)],
            cwd=self.contracts,
            env={**os.environ, "HOME": str(self.home), "TMPDIR": str(self.temp)},
            capture_output=True,
            text=True,
        )

    def configure(self, contents):
        (self.contracts / "Makefile").write_text(contents)

    def git(self, repo, *args):
        return subprocess.check_output(
            ["git", "-C", str(repo), *args], stderr=subprocess.DEVNULL, text=True
        ).strip()

    def create_assembler(self):
        repo = self.root / "assembler source"
        repo.mkdir()
        self.git(repo, "init")
        self.git(repo, "config", "user.name", "Installer Test")
        self.git(repo, "config", "user.email", "test@example.com")
        self.git(repo, "config", "commit.gpgsign", "false")
        (repo / "Makefile").write_text(
            "bin/evm: evm.sh\n\tmkdir -p bin\n\tcp evm.sh bin/evm\n"
        )
        revisions = []
        for version in ("first", "second"):
            (repo / "evm.sh").write_text(f"#!/bin/sh\necho {version}\n")
            self.git(repo, "add", ".")
            self.git(repo, "commit", "-m", version)
            revisions.append(self.git(repo, "rev-parse", "HEAD"))
        return repo, revisions

    def test_installs_each_checkouts_revision(self):
        repo, revisions = self.create_assembler()
        self.configure("include assembler.mk\n")
        for revision, expected in zip(revisions, ("first", "second")):
            with self.subTest(revision=revision):
                (self.contracts / "assembler.mk").write_text(
                    f"EVM_REPO := {repo}\nEVM_REVISION := {revision}\n"
                )
                result = self.run_installer()
                self.assertEqual(result.returncode, 0, result.stderr)
                executable = self.home / ".local/bin/evm"
                self.assertEqual(
                    subprocess.check_output([str(executable)], text=True).strip(),
                    expected,
                )
                self.assertEqual(list(self.temp.iterdir()), [])

    def test_legacy_checkout_skips_installation_and_default_target(self):
        self.configure("build:\n\texit 1\n")
        result = self.run_installer()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.home / ".local/bin/evm").exists())

    def test_partial_configuration_fails(self):
        for config in ("EVM_REPO := missing\n", "EVM_REVISION := missing\n"):
            with self.subTest(config=config):
                self.configure(config)
                result = self.run_installer()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("filecoin-services must define", result.stderr)

    def test_make_errors_are_reported(self):
        self.configure("$(error broken configuration)\n")
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("broken configuration", result.stderr)

    def test_checkout_failure_is_reported_and_cleaned_up(self):
        repo, _ = self.create_assembler()
        self.configure(f"EVM_REPO := {repo}\nEVM_REVISION := nonexistent\n")
        result = self.run_installer()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("nonexistent", result.stderr)
        self.assertFalse((self.home / ".local/bin/evm").exists())
        self.assertEqual(list(self.temp.iterdir()), [])


if __name__ == "__main__":
    unittest.main()
