#!/usr/bin/env python3
"""Run selected scenario tests against a locally started foc-devnet.

Separately installed dependencies:
  - Docker with the current non-root user able to run Docker commands.
  - Rust toolchain with cargo, used to build the foc-devnet CLI.
  - git, node, pnpm, curl, tar, and sudo.
  - Python 3.11+ for this runner and the scenario scripts.
  - /etc/hosts entry for host.docker.internal. If it is missing, this runner
    tries to add "127.0.0.1 host.docker.internal" with sudo.

Dependencies managed by scripts/setup-scenarios-prerequisites.sh:
  - Foundry cast/forge under ~/.foc-devnet/artifacts/foundry/bin.
  - Python 3.11.10 via pyenv for cqlsh.
  - Apache Cassandra cqlsh under ~/.foc-devnet/artifacts/cassandra.

Examples:
  scripts/run-local-scenarios.py test_containers test_basic_balances
  scripts/run-local-scenarios.py --all
  scripts/run-local-scenarios.py --clean --keep-running test_multi_copy_upload
  scripts/run-local-scenarios.py --skip-init test_multi_copy_upload

The setup and teardown steps are intentionally kept in separate functions so
they can be moved into shared local-dev helpers later.
"""

from __future__ import annotations

import argparse
import os
import shlex
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO_ROOT))

from scenarios.report import write_report  # noqa: E402
from scenarios.run import ORDER, _print_summary, _run_single_test  # noqa: E402

DEFAULT_BINARY = REPO_ROOT / "target" / "release" / "foc-devnet"
SCENARIO_TIMEOUTS = dict(ORDER)
RUNTIME_DOCKER_IMAGES = (
    ("foc-lotus", REPO_ROOT / "docker" / "lotus" / "Dockerfile", REPO_ROOT),
    ("foc-lotus-miner", REPO_ROOT / "docker" / "lotus-miner" / "Dockerfile", REPO_ROOT),
    ("foc-curio", REPO_ROOT / "docker" / "curio" / "Dockerfile", REPO_ROOT),
    (
        "foc-yugabyte",
        REPO_ROOT / "docker" / "yugabyte" / "Dockerfile",
        Path.home() / ".foc-devnet" / "artifacts",
    ),
)


def run_command(cmd: list[str], *, env: dict[str, str] | None = None) -> None:
    print(f"+ {shlex.join(cmd)}", flush=True)
    subprocess.run(cmd, cwd=REPO_ROOT, env=env, check=True)


def command_succeeds(cmd: list[str]) -> bool:
    return (
        subprocess.run(
            cmd, cwd=REPO_ROOT, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL
        ).returncode
        == 0
    )


def ensure_host_docker_internal() -> None:
    hosts = Path("/etc/hosts").read_text()
    if "host.docker.internal" in hosts:
        return

    run_command(
        [
            "sudo",
            "sh",
            "-c",
            "printf '\\n127.0.0.1 host.docker.internal\\n' >> /etc/hosts",
        ]
    )


def install_scenario_prerequisites() -> None:
    run_command([str(REPO_ROOT / "scripts" / "setup-scenarios-prerequisites.sh")])


def build_foc_devnet() -> Path:
    run_command(["cargo", "build", "--release"])
    return DEFAULT_BINARY


def clean_devnet(binary: Path, *, all_config: bool) -> None:
    cmd = [str(binary), "clean"]
    if all_config:
        cmd.append("--all")
    run_command(cmd)


def init_devnet(binary: Path, init_flags: str) -> None:
    run_command([str(binary), "init", *shlex.split(init_flags)])


def image_exists(image: str) -> bool:
    return command_succeeds(["docker", "image", "inspect", image])


def build_runtime_docker_images() -> None:
    uid = str(os.getuid())
    gid = str(os.getgid())

    for image, dockerfile, context in RUNTIME_DOCKER_IMAGES:
        if image_exists(image):
            print(
                f"[INFO] Docker image {image} already exists, skipping build",
                flush=True,
            )
            continue

        if image == "foc-yugabyte" and not (context / "yugabyte").is_dir():
            raise SystemExit(
                f"Missing Yugabyte artifact directory: {context / 'yugabyte'}\n"
                "Run foc-devnet init once, or provide the artifact before using --skip-init."
            )

        run_command(
            [
                "docker",
                "build",
                "--progress",
                "plain",
                "--build-arg",
                f"USER_ID={uid}",
                "--build-arg",
                f"GROUP_ID={gid}",
                "-f",
                str(dockerfile),
                "-t",
                image,
                str(context),
            ]
        )


def build_components(binary: Path) -> None:
    run_command([str(binary), "build", "lotus"])
    run_command([str(binary), "build", "curio"])


def start_devnet(binary: Path, *, parallel: bool) -> None:
    cmd = [str(binary), "start"]
    if parallel:
        cmd.append("--parallel")
    run_command(cmd)


def stop_devnet(binary: Path) -> None:
    run_command([str(binary), "stop"])


def normalize_scenario_name(name: str) -> str:
    return Path(name).stem


def select_scenarios(names: list[str], *, all_scenarios: bool) -> list[tuple[str, int]]:
    if all_scenarios:
        return ORDER

    if not names:
        raise SystemExit("Pass at least one scenario name, or use --all.")

    selected = {normalize_scenario_name(name) for name in names}
    unknown = sorted(selected - set(SCENARIO_TIMEOUTS))
    if unknown:
        known = ", ".join(name for name, _ in ORDER)
        raise SystemExit(f"Unknown scenario(s): {', '.join(unknown)}. Known: {known}")

    return [(name, timeout) for name, timeout in ORDER if name in selected]


def run_selected_scenarios(
    scenarios: list[tuple[str, int]], env: dict[str, str]
) -> int:
    os.environ.update(env)
    started = time.time()
    results = []

    for name, timeout in scenarios:
        scenario_file = REPO_ROOT / "scenarios" / f"{name}.py"
        result = _run_single_test(str(scenario_file), name, timeout)
        results.append(result)

    elapsed = int(time.time() - started)
    _print_summary(results, elapsed)
    print(f"Report: {write_report(results=results, elapsed=elapsed)}")
    return 0 if all(result.is_passed for result in results) else 1


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Set up a local foc-devnet, run selected scenarios, then stop it."
    )
    parser.add_argument("scenarios", nargs="*", help="Scenario names to run.")
    parser.add_argument("--all", action="store_true", help="Run all scenarios.")
    parser.add_argument(
        "--init-flags",
        default="",
        help="Extra flags forwarded to foc-devnet init, parsed with shell syntax.",
    )
    parser.add_argument(
        "--binary",
        type=Path,
        default=None,
        help="Use an existing foc-devnet binary instead of building target/release.",
    )
    parser.add_argument(
        "--clean",
        action="store_true",
        help="Run foc-devnet clean before init, preserving config.toml.",
    )
    parser.add_argument(
        "--clean-all",
        action="store_true",
        help="Run foc-devnet clean --all before init.",
    )
    parser.add_argument(
        "--skip-prerequisites",
        action="store_true",
        help="Skip scripts/setup-scenarios-prerequisites.sh.",
    )
    parser.add_argument("--skip-init", action="store_true", help="Skip init.")
    parser.add_argument(
        "--skip-image-build",
        action="store_true",
        help="Skip runtime Docker image builds normally covered by init.",
    )
    parser.add_argument(
        "--skip-component-build",
        action="store_true",
        help="Skip foc-devnet build lotus/curio.",
    )
    parser.add_argument("--skip-start", action="store_true", help="Skip start.")
    parser.add_argument(
        "--no-parallel-start",
        action="store_true",
        help="Start without --parallel.",
    )
    parser.add_argument(
        "--keep-running",
        action="store_true",
        help="Do not run foc-devnet stop after scenarios.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    selected = select_scenarios(args.scenarios, all_scenarios=args.all)
    env = {**os.environ, "SCENARIO_RUN_TYPE": "local-selected"}
    binary = args.binary.resolve() if args.binary else None
    needs_binary = any(
        [
            args.clean,
            args.clean_all,
            not args.skip_init,
            not args.skip_component_build,
            not args.skip_start,
        ]
    )
    started = False

    try:
        if not args.skip_prerequisites:
            install_scenario_prerequisites()

        ensure_host_docker_internal()

        if binary is None and needs_binary:
            binary = build_foc_devnet()

        if args.clean or args.clean_all:
            assert binary is not None
            clean_devnet(binary, all_config=args.clean_all)

        if not args.skip_init:
            assert binary is not None
            init_devnet(binary, args.init_flags)
        elif not args.skip_image_build:
            build_runtime_docker_images()

        if not args.skip_component_build:
            assert binary is not None
            build_components(binary)

        if not args.skip_start:
            assert binary is not None
            started = True
            start_devnet(binary, parallel=not args.no_parallel_start)

        return run_selected_scenarios(selected, env)
    finally:
        if started and not args.keep_running and binary is not None:
            stop_devnet(binary)


if __name__ == "__main__":
    sys.exit(main())
