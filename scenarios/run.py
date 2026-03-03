#!/usr/bin/env python3
# core.py — assertions, devnet-info helpers, test runner, and reporting.
# Run all tests:  python3 core.py
# Run one test:   python3 test_containers.py
import os, sys, json, subprocess, importlib.util, time

# Ensure the project root (parent of scenarios_py/) is on sys.path so that
# test files can do `from scenarios_py.run import *` regardless of cwd.
_project_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if _project_root not in sys.path:
    sys.path.insert(0, _project_root)

# Allow test files to `from core import *` even when core runs as __main__.
sys.modules.setdefault("core", sys.modules[__name__])

DEVNET_INFO = os.environ.get("DEVNET_INFO", os.path.expanduser("~/.foc-devnet/state/latest/devnet-info.json"))
REPORT_MD   = os.environ.get("REPORT_FILE",  os.path.expanduser("~/.foc-devnet/state/latest/scenario_report.md"))

# ── Scenario execution order (mirrors scenarios/order.sh) ────
ORDER = [
    "test_containers",
    "test_basic_balances",
    "test_storage_e2e",
]

_pass = 0
_fail = 0

# ── Logging ──────────────────────────────────────────────────

def info(msg):
    print(f"[INFO] {msg}")

def ok(msg):
    global _pass
    print(f"[ OK ] {msg}")
    _pass += 1

def fail(msg):
    global _fail
    print(f"[FAIL] {msg}", file=sys.stderr)
    _fail += 1

# ── Assertions ───────────────────────────────────────────────

def assert_eq(a, b, msg):
    if a == b: ok(msg)
    else: fail(f"{msg} (got '{a}', want '{b}')")

def assert_gt(a, b, msg):
    try:
        if int(a) > int(b): ok(msg)
        else: fail(f"{msg} (got '{a}', want > '{b}')")
    except: fail(f"{msg} (not an int: '{a}')")

def assert_not_empty(v, msg):
    if v: ok(msg)
    else: fail(f"{msg} (empty)")

def assert_ok(cmd, msg):
    if subprocess.call(cmd, shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL) == 0: ok(msg)
    else: fail(msg)

# ── Shell helpers ─────────────────────────────────────────────

def sh(cmd):
    """Run cmd in a shell and return stdout stripped, or '' on error."""
    return subprocess.run(cmd, shell=True, text=True, capture_output=True).stdout.strip()

def devnet_info():
    """Load devnet-info.json as a dict."""
    with open(DEVNET_INFO) as f:
        return json.load(f)

def ensure_foundry():
    """Install Foundry if cast is not on PATH."""
    if subprocess.call("command -v cast", shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL) != 0:
        info("Installing Foundry...")
        os.system("curl -sSL https://foundry.paradigm.xyz | bash")
        os.environ["PATH"] = os.path.expanduser("~/.foundry/bin:") + os.environ["PATH"]
        os.system(os.path.expanduser("~/.foundry/bin/foundryup"))
    assert_ok("command -v cast", "cast is installed")

# ── Runner ────────────────────────────────────────────────────

def run_tests():
    """Run scenarios in ORDER. Returns list of (name, passed, failed)."""
    global _pass, _fail
    here = os.path.dirname(os.path.abspath(__file__))
    results = []
    for name in ORDER:
        path = os.path.join(here, f"{name}.py")
        _pass = _fail = 0
        info(f"=== {name} ===")
        try:
            spec = importlib.util.spec_from_file_location(name, path)
            mod = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(mod)
            mod.run()
        except Exception as e:
            fail(f"unhandled exception: {e}")
        results.append((name, _pass, _fail))
    return results

# ── Reporting ─────────────────────────────────────────────────

def write_report(results, elapsed):
    """Write a markdown report to REPORT_DIR. Returns path written."""
    total_assert_pass = sum(p for _, p, _ in results)
    total_assert_fail = sum(f for _, _, f in results)
    total_scenarios = len(results)
    scenario_pass = sum(1 for _, p, f in results if f == 0)
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
        fh.write(f"| Total Scenarios | {total_scenarios} |\n| Scenarios Passed | {scenario_pass} |\n| Scenarios Failed | {scenario_fail} |\n")
        fh.write(f"| Total Assertions | {total_assert_pass+total_assert_fail} |\n| Assertions Passed | {total_assert_pass} |\n| Assertions Failed | {total_assert_fail} |\n| Duration | {elapsed}s |\n\n")
        fh.write("## Details\n\n| Status | Scenario | Passed | Failed |\n|--------|----------|--------|--------|\n")
        for name, p, f in results:
            icon = "✅" if f == 0 else "❌"
            fh.write(f"| {icon} {'PASS' if f == 0 else 'FAIL'} | {name} | {p} | {f} |\n")
    return REPORT_MD

if __name__ == "__main__":
    start = time.time()
    results = run_tests()
    elapsed = int(time.time() - start)

    total_assert_pass = sum(p for _, p, _ in results)
    total_assert_fail = sum(f for _, _, f in results)
    total_scenarios = len(results)
    scenario_pass = sum(1 for _, _, f in results if f == 0)
    scenario_fail = total_scenarios - scenario_pass
    print(f"\n{'='*50}")
    print(f"Scenarios: {total_scenarios}  Passed: {scenario_pass}  Failed: {scenario_fail}  ({elapsed}s)")
    print(f"Assertions: {total_assert_pass+total_assert_fail}  Passed: {total_assert_pass}  Failed: {total_assert_fail}")

    report = write_report(results, elapsed)
    print(f"Report: {report}")
    # Print CI run URL in stdout if available
    if os.environ.get("GITHUB_RUN_ID") and os.environ.get("GITHUB_REPOSITORY"):
        github_server = os.environ.get("GITHUB_SERVER_URL", "https://github.com")
        ci_url = f"{github_server}/{os.environ.get('GITHUB_REPOSITORY')}/actions/runs/{os.environ.get('GITHUB_RUN_ID')}"
        print(f"CI Run: {ci_url}")
    sys.exit(0 if scenario_fail == 0 else 1)
