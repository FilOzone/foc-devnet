#!/usr/bin/env python3
# core.py — assertions, devnet-info helpers, test runner, and reporting.
# Run all tests:  python3 core.py
# Run one test:   python3 test_containers.py
import os
import sys
import json
import subprocess
import threading
import queue
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
    ("test_storage_e2e", 100),
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


def _read_stream(stream, q, label):
    """Read lines from a subprocess stream and enqueue them with a label."""
    try:
        for line in stream:
            q.put((label, line.rstrip("\n")))
    except ValueError:
        pass  # Pipe closed
    finally:
        q.put((label, None))  # Sentinel to signal stream EOF


def run_tests():
    """Run scenarios in ORDER. Returns list of (name, passed, elapsed_time, log_lines, timed_out)."""
    here = os.path.dirname(os.path.abspath(__file__))
    results = []
    for name, timeout_sec in ORDER:
        path = os.path.join(here, f"{name}.py")
        info(f"=== {name} (timeout: {timeout_sec}s) ===")
        test_start = time.time()
        # Run the test in a subprocess, capturing stdout and stderr separately
        process = subprocess.Popen(
            [sys.executable, path],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,  # Line buffered
        )
        q = queue.Queue()
        stdout_lines = []
        stderr_lines = []
        timed_out = False
        # Reader threads for non-blocking stdout/stderr capture
        t_out = threading.Thread(
            target=_read_stream, args=(process.stdout, q, "stdout"), daemon=True
        )
        t_err = threading.Thread(
            target=_read_stream, args=(process.stderr, q, "stderr"), daemon=True
        )
        t_out.start()
        t_err.start()
        try:
            streams_done = 0
            while streams_done < 2:
                remaining = timeout_sec - (time.time() - test_start)
                if remaining <= 0:
                    timed_out = True
                    process.kill()
                    break
                try:
                    label, line = q.get(timeout=min(remaining, 1.0))
                    if line is None:
                        streams_done += 1
                        continue
                    if label == "stdout":
                        print(line)
                        stdout_lines.append(line)
                    else:
                        print(f"  [stderr] {line}", file=sys.stderr)
                        stderr_lines.append(line)
                except queue.Empty:
                    if process.poll() is not None and q.empty():
                        break
                    continue
            # Wait for reader threads to finish and drain remaining queue
            t_out.join(timeout=3)
            t_err.join(timeout=3)
            while not q.empty():
                try:
                    label, line = q.get_nowait()
                    if line is None:
                        continue
                    if label == "stdout":
                        print(line)
                        stdout_lines.append(line)
                    else:
                        print(f"  [stderr] {line}", file=sys.stderr)
                        stderr_lines.append(line)
                except queue.Empty:
                    break
            if timed_out:
                timeout_msg = (
                    f"[TIMEOUT] Test '{name}' exceeded {timeout_sec}s limit "
                    f"— {len(stdout_lines)} stdout and {len(stderr_lines)} stderr lines captured"
                )
                print(timeout_msg, file=sys.stderr)
                stdout_lines.append(timeout_msg)
            try:
                return_code = process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
                return_code = -1
        except Exception as e:
            error_msg = f"[ERROR] Exception during test execution: {e}"
            print(error_msg)
            stdout_lines.append(error_msg)
            process.kill()
            process.wait()
            return_code = -1
        elapsed_time = int(time.time() - test_start)
        # Combine stdout and stderr into log_lines for the report
        log_lines = stdout_lines.copy()
        if stderr_lines:
            log_lines.append("")
            log_lines.append("--- stderr ---")
            log_lines.extend(stderr_lines)
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
        # Version info from foc-devnet version
        version_info = get_version_info()
        fh.write("## Version Info\n\n")
        fh.write(f"```\n{version_info}\n```\n\n")
        fh.write("## Summary\n\n")
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
