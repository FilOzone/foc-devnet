#!/usr/bin/env python3
# core.py — assertions, devnet-info helpers, test runner, and reporting.
# Run all tests:  python3 core.py
# Run one test:   python3 test_containers.py
import os
import sys
import json
import subprocess
import time

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
    ("test_containers", 5),
    ("test_basic_balances", 10),
    ("test_storage_e2e", 50),
    ("test_caching_subsystem", 90),
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


# ── Runner ────────────────────────────────────────────────────


def run_tests():
    """Run scenarios in ORDER. Returns list of (name, passed, elapsed_time, log_lines, timed_out)."""
    here = os.path.dirname(os.path.abspath(__file__))
    results = []
    for name, timeout in ORDER:
        path = os.path.join(here, f"{name}.py")
        info(f"=== {name} (timeout: {timeout}s) ===")
        test_start = time.time()
        # Run the test in a subprocess, capturing output while also displaying it live
        process = subprocess.Popen(
            [sys.executable, path],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            bufsize=1,  # Line buffered
        )
        log_lines = []
        timed_out = False
        try:
            # Read output line by line with timeout detection
            while True:
                # Check if timeout exceeded
                if time.time() - test_start > timeout:
                    timed_out = True
                    process.kill()
                    timeout_msg = f"[TIMEOUT] Test exceeded {timeout}s limit"
                    print(timeout_msg)
                    log_lines.append(timeout_msg)
                    break
                # Try to read a line (non-blocking with select would be better, but this works)
                line = process.stdout.readline()
                if not line:
                    break  # EOF reached
                line = line.rstrip("\n")
                print(line)  # Display live
                log_lines.append(line)  # Capture for report
            # Wait for process to complete (or confirm it's dead)
            try:
                return_code = process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
                return_code = -1
        except Exception as e:
            error_msg = f"[ERROR] Exception during test execution: {e}"
            print(error_msg)
            log_lines.append(error_msg)
            process.kill()
            process.wait()
            return_code = -1
        elapsed_time = int(time.time() - test_start)
        # Determine pass/fail based on return code and timeout
        passed = return_code == 0 and not timed_out
        results.append((name, passed, elapsed_time, log_lines, timed_out))
    return results


# ── Reporting ─────────────────────────────────────────────────


def write_report(results, elapsed):
    """Write a markdown report to REPORT_MD. Returns path written."""
    total_scenarios = len(results)
    scenario_pass = sum(1 for _, passed, _, _, _ in results if passed)
    scenario_fail = total_scenarios - scenario_pass
    with open(REPORT_MD, "w") as fh:
        fh.write("# Scenario Test Report\n\n")
        # If running in GitHub Actions, include a link to the run
        github_run_id = os.environ.get("GITHUB_RUN_ID")
        github_repo = os.environ.get("GITHUB_REPOSITORY")
        if github_run_id and github_repo:
            github_server = os.environ.get("GITHUB_SERVER_URL", "https://github.com")
            ci_url = f"{github_server}/{github_repo}/actions/runs/{github_run_id}"
            fh.write(f"**CI Run**: [{ci_url}]({ci_url})\n\n")
        fh.write("| Metric | Value |\n|--------|-------|\n")
        fh.write(
            f"| Total Scenarios | {total_scenarios} |\n| Scenarios Passed | {scenario_pass} |\n| Scenarios Failed | {scenario_fail} |\n"
        )
        fh.write(f"| Duration | {elapsed}s |\n\n")
        fh.write("## Test Results\n\n")
        for name, passed, test_time, logs, timed_out in results:
            icon = "✅" if passed else "❌"
            if timed_out:
                status = f"TIMEOUT ({test_time}s)"
            else:
                status = f"{'PASS' if passed else 'FAIL'} ({test_time}s)"
            fh.write(
                f"<details>\n<summary>{icon} <b>{name}</b> - {status}</summary>\n\n```\n"
            )
            fh.write("\n".join(logs))
            fh.write("\n```\n</details>\n\n")
    return REPORT_MD


if __name__ == "__main__":
    start = time.time()
    results = run_tests()
    elapsed = int(time.time() - start)

    total_scenarios = len(results)
    scenario_pass = sum(1 for _, passed, _, _, _ in results if passed)
    scenario_fail = total_scenarios - scenario_pass
    print(f"\n{'='*50}")
    print(
        f"Scenarios: {total_scenarios}  Passed: {scenario_pass}  Failed: {scenario_fail}  ({elapsed}s)"
    )
    # Show individual test timings
    for name, passed, test_time, _, timed_out in results:
        status_icon = "✅" if passed else "❌"
        status_text = "TIMEOUT" if timed_out else ("PASS" if passed else "FAIL")
        print(f"  {status_icon} {name}: {status_text} ({test_time}s)")

    report = write_report(results, elapsed)
    print(f"Report: {report}")
    # Print CI run URL in stdout if available
    if os.environ.get("GITHUB_RUN_ID") and os.environ.get("GITHUB_REPOSITORY"):
        github_server = os.environ.get("GITHUB_SERVER_URL", "https://github.com")
        ci_url = f"{github_server}/{os.environ.get('GITHUB_REPOSITORY')}/actions/runs/{os.environ.get('GITHUB_RUN_ID')}"
        print(f"CI Run: {ci_url}")
    sys.exit(0 if scenario_fail == 0 else 1)
