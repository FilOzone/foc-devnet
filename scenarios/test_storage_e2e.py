#!/usr/bin/env python3
import os
import sys
import tempfile

# Ensure the project root (parent of scenarios/) is on sys.path
_project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _project_root not in sys.path:
    sys.path.insert(0, _project_root)

from scenarios.run import *

SYNAPSE_SDK_REPO = "https://github.com/FilOzone/synapse-sdk/"
RAND_FILE_NAME = "random_file"
RAND_FILE_SIZE = 20 * 1024 * 1024
RAND_FILE_SEED = 42


def run():
    assert_ok("command -v git", "git is installed")
    assert_ok("command -v node", "node is installed")
    assert_ok("command -v pnpm", "pnpm is installed")

    with tempfile.TemporaryDirectory(prefix="synapse-sdk-") as temp_dir:
        sdk_dir = Path(temp_dir) / "synapse-sdk"

        info(f"--- Cloning synapse-sdk to {sdk_dir} ---")
        if not run_cmd(
            ["git", "clone", SYNAPSE_SDK_REPO, str(sdk_dir)], label="synapse-sdk cloned"
        ):
            return

        info("--- Checking out synapse-sdk @ master (latest) ---")
        if not run_cmd(
            ["git", "checkout", "master"],
            cwd=str(sdk_dir),
            label="synapse-sdk checked out at master head",
        ):
            return

        sdk_commit = sh(f"git -C {sdk_dir} rev-parse HEAD")
        info(f"synapse-sdk commit: {sdk_commit}")

        info("--- Installing synapse-sdk dependencies with pnpm ---")
        if not run_cmd(
            ["pnpm", "install"], cwd=str(sdk_dir), label="pnpm install completed"
        ):
            return

        info("--- Building synapse-sdk TypeScript packages ---")
        if not run_cmd(
            ["pnpm", "build"], cwd=str(sdk_dir), label="pnpm build completed"
        ):
            return

        random_file = sdk_dir / RAND_FILE_NAME
        info(f"--- Creating random file ({RAND_FILE_SIZE} bytes) ---")
        write_random_file(random_file, RAND_FILE_SIZE, RAND_FILE_SEED)
        actual_size = random_file.stat().st_size
        assert_eq(
            actual_size,
            RAND_FILE_SIZE,
            f"{RAND_FILE_NAME} created with exact size {RAND_FILE_SIZE} bytes",
        )

        info("--- Running Synapse SDK storage e2e script against devnet ---")
        cmd_env = os.environ.copy()
        cmd_env["NETWORK"] = "devnet"
        run_cmd(
            ["node", "utils/example-storage-e2e.js", RAND_FILE_NAME],
            cwd=str(sdk_dir),
            env=cmd_env,
            label="NETWORK=devnet node utils/example-storage-e2e.js random_file",
            print_output=True,
        )


if __name__ == "__main__":
    run()
