#!/usr/bin/env python3
# core.py — assertions, devnet-info helpers, test runner, and reporting.
# Run all tests:  python3 core.py
# Run one test:   python3 test_containers.py
import os
import sys
import json
import subprocess
import time
from dataclasses import dataclass
from string import Template
import datetime

# Ensure the project root (parent of scenarios_py/) is on sys.path so that
# test files can do `from scenarios_py.run import *` regardless of cwd.
_project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _project_root not in sys.path:
    sys.path.insert(0, _project_root)

# Allow test files to `from core import *` even when core runs as __main__.
sys.modules.setdefault("core", sys.modules[__name__])

DEVNET_INFO = os.environ.get(
    "DEVNET_INFO", os.path.expanduser("~/.foc-devnet/state/latest/devnet-info.json")
)
REPORT_MD = os.environ.get(
    "REPORT_FILE", os.path.expanduser("~/.foc-devnet/state/latest/scenario_report.md")
)

# ── Scenario execution order (mirrors scenarios/order.sh) ────
# Each entry is (test_name, timeout_seconds)
ORDER = [
    # ("test_containers", 5),
    # ("test_basic_balances", 10),
    # ("test_storage_e2e", 100),
    ("test_caching_subsystem", 200),
]

_pass = 0
_fail = 0
_log_lines: list = []

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
    "fail logs a failure and exits the scenario entirely with exit code = 1"
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
    except:
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


def ensure_foundry():
    """Install Foundry if cast is not on PATH."""
    if (
        subprocess.call(
            "command -v cast",
            shell=True,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        != 0
    ):
        info("Installing Foundry...")
        os.system("curl -sSL https://foundry.paradigm.xyz | bash")
        os.environ["PATH"] = os.path.expanduser("~/.foundry/bin:") + os.environ["PATH"]
        os.system(os.path.expanduser("~/.foundry/bin/foundryup"))
    assert_ok("command -v cast", "cast is installed")


# ── Version info ──────────────────────────────────────────────


def get_version_info():
    """Capture output of `foc-devnet version` for inclusion in reports."""
    for binary in ["./foc-devnet", "foc-devnet"]:
        try:
            result = subprocess.run(
                [binary, "version"],
                capture_output=True,
                text=True,
                timeout=10,
            )
            if result.returncode == 0:
                return result.stdout.strip()
        except (FileNotFoundError, subprocess.TimeoutExpired):
            continue
    return "foc-devnet version: not available"


# ── Runner ────────────────────────────────────────────────────


@dataclass
class TestResult:
    test_name: str
    is_passed: bool
    time_taken_sec: int
    timeout_sec: int
    log_lines: list[str]
    return_code: int


def run_tests():
    """Run scenarios in ORDER. Returns list of (name, passed, elapsed_time, log_lines, timed_out)."""
    pwd = os.path.dirname(os.path.abspath(__file__))
    results: list[TestResult] = []

    for name, timeout_sec in ORDER:
        scenario_py_file = os.path.join(pwd, f"{name}.py")

        info(f"=== {name} (timeout: {timeout_sec}s) ===")
        test_start = time.time()
        # Run the test in a subprocess, merging stderr into stdout for correct ordering
        process = subprocess.Popen(
            [sys.executable, scenario_py_file],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        timed_out = False
        try:
            stdout, _ = process.communicate(timeout=timeout_sec)
            return_code = process.returncode
        except subprocess.TimeoutExpired:
            process.kill()
            stdout, _ = process.communicate()
            timed_out = True
            return_code = -1
        except Exception as e:
            process.kill()
            process.wait()
            stdout = f"[ERROR] Exception during test execution: {e}"
            return_code = -1

        log_lines = stdout.splitlines()
        if timed_out:
            log_lines.append(
                f"[TIMEOUT] Test '{name}' exceeded {timeout_sec}s limit "
                f"— {len(log_lines)} lines captured"
            )
        for log_line in log_lines:
            print(f"    {log_line}")
        elapsed_time = int(time.time() - test_start)
        passed = return_code == 0 and not timed_out
        results.append(
            TestResult(
                test_name=name,
                is_passed=passed,
                time_taken_sec=elapsed_time,
                timeout_sec=timeout_sec,
                log_lines=log_lines,
                return_code=return_code,
            )
        )
    return results


# ── Reporting ─────────────────────────────────────────────────

report_template = Template("""
# Scenarios Tests 

| Description | Data                                                                |
|-------------| ------------------------------------------------------------------- |
| Type        | **$run_type**                                                           |
| Date        | $date                                                               |
| Status.     | PASS ✅:**$pass_count**, FAIL 🟥:**$fail_count**, Total:$total_count |
| CI run      | $ci_run_link                                                        |

## Versions info
$version_info

## Tests summary
$test_summary
""")


def write_report(results: list[TestResult] = [], elapsed: int = 0):
    """Write a markdown report to REPORT_MD. Returns path written."""
    _type = os.environ.get("SCENARIO_RUN_TYPE", "local")
    total = len(results)
    passed = sum(1 for r in results if r.is_passed)
    failed = total - passed

    github_run_id = os.environ.get("GITHUB_RUN_ID")
    github_repo = os.environ.get("GITHUB_REPOSITORY")
    if github_run_id and github_repo:
        github_server = os.environ.get("GITHUB_SERVER_URL", "https://github.com")
        ci_url = f"{github_server}/{github_repo}/actions/runs/{github_run_id}"
        ci_run_link = f"[{ci_url}]({ci_url})"
    else:
        ci_run_link = "-"

    test_summary_parts = []
    for r in results:
        timed_out = r.return_code != 0 and r.time_taken_sec >= r.timeout_sec
        icon = "✅" if r.is_passed else "❌"
        status = (
            f"TIMEOUT ({r.time_taken_sec}s)"
            if timed_out
            else f"{'PASS' if r.is_passed else 'FAIL'} ({r.time_taken_sec}s)"
        )
        test_summary_parts.append(
            f"<details>\n<summary>{icon} <b>{r.test_name}</b> - {status}</summary>\n\n"
            f"```\n{chr(10).join(r.log_lines)}\n```\n</details>"
        )

    content = report_template.substitute(
        run_type=_type,
        date=datetime.datetime.now(datetime.UTC).strftime("%d-%B-%Y %H:%M:%S GMT +0"),
        pass_count=passed,
        fail_count=failed,
        total_count=total,
        ci_run_link=ci_run_link,
        version_info=f"```\n{get_version_info()}\n```",
        test_summary="\n\n".join(test_summary_parts),
    )

    with open(REPORT_MD, "w") as fh:
        fh.write(content)
    return REPORT_MD


if __name__ == "__main__":
    start = time.time()
    results = run_tests()
    elapsed = int(time.time() - start)

    total_scenarios = len(results)
    scenario_pass = sum(1 for r in results if r.is_passed)
    scenario_fail = total_scenarios - scenario_pass
    print(f"\n{'='*50}")
    print(
        f"Scenarios: {total_scenarios}  Passed: {scenario_pass}  Failed: {scenario_fail}  ({elapsed}s)"
    )
    for r in results:
        timed_out = r.return_code != 0 and r.time_taken_sec >= r.timeout_sec
        status_icon = "✅" if r.is_passed else "❌"
        status_text = "TIMEOUT" if timed_out else ("PASS" if r.is_passed else "FAIL")
        print(f"  {status_icon} {r.test_name}: {status_text} ({r.time_taken_sec}s)")

    report = write_report(results=results)
    print(f"Report: {report}")
    # Print CI run URL in stdout if available
    if os.environ.get("GITHUB_RUN_ID") and os.environ.get("GITHUB_REPOSITORY"):
        github_server = os.environ.get("GITHUB_SERVER_URL", "https://github.com")
        ci_url = f"{github_server}/{os.environ.get('GITHUB_REPOSITORY')}/actions/runs/{os.environ.get('GITHUB_RUN_ID')}"
        print(f"CI Run: {ci_url}")
    sys.exit(0 if scenario_fail == 0 else 1)
