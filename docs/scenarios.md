# Scenario tests

Scenario tests run serially against one running devnet. The Python runner uses
the standard library, but scenarios also invoke Docker and Foundry and run
Node.js clients. Use Python 3.11+, Git, Node.js 24+, npm, and pnpm. The
[prerequisite script](../scripts/setup-scenarios-prerequisites.sh) checks the
JavaScript tools and installs Foundry; the scenarios install client dependencies
and need network access for downloads.
Use the default provider configuration for the full suite: the replication
scenarios need two usable storage providers.

### Running scenarios

From the repository root, after starting the devnet and confirming that its
export is present:

```bash
./scripts/setup-scenarios-prerequisites.sh
python3 scenarios/run.py

# Or run a single scenario without the suite report
python3 scenarios/test_basic_balances.py
```

By default, the suite reads `state/latest/devnet-info.json`. To select an
export explicitly, set both `DEVNET_INFO` (Python checks) and `DEVNET_INFO_PATH`
(Synapse and the [pinned filecoin-pin version](../ci/dependency-profiles.json))
to the same file. The services for that run must still be running.

The suite writes `~/.foc-devnet/state/latest/scenario_report.md` by default,
independently of `FOC_DEVNET_BASEDIR`. This example uses `jq` to select the
current run explicitly and write its report alongside its export. Use an
existing directory for `REPORT_FILE`.

```bash
base="${FOC_DEVNET_BASEDIR:-$HOME/.foc-devnet}"
run_id=$(jq -er '.run_id' "$base/state/current_runid.json") &&
info="$base/run/$run_id/devnet-info.json" &&
test -f "$info" &&
PATH="$HOME/.foc-devnet/artifacts/foundry/bin:$PATH" \
  FOC_DEVNET_BASEDIR="$base" \
  DEVNET_INFO="$info" DEVNET_INFO_PATH="$info" \
  REPORT_FILE="$base/run/$run_id/scenario_report.md" \
  python3 scenarios/run.py
```

The prerequisite script installs Foundry under `~/.foc-devnet` regardless of
the selected base directory. Adding that Foundry directory to `PATH`, as in
the example above, makes it available to scenarios using a custom base directory.

### CI integration

CI runs scenarios after startup and uploads the report as a workflow artifact.
Nightly or manually enabled reporting can also create or update a GitHub issue.
See [CI Dependency Profiles](../ci/README.md) for dependency selection and pins.
