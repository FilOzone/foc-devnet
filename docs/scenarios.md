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

The suite expects the running devnet's `state/latest/devnet-info.json`.
Individual Python checks accept `DEVNET_INFO`; the Synapse Node.js checks
accept `DEVNET_INFO_PATH`. Set both to the same export for a Synapse scenario
that uses that export. These options do not configure filecoin-pin.

The suite writes `~/.foc-devnet/state/latest/scenario_report.md` by default,
independently of `FOC_DEVNET_BASEDIR`. To write the report elsewhere, use `jq`
to read the current run ID:

```bash
base="${FOC_DEVNET_BASEDIR:-$HOME/.foc-devnet}"
run_id=$(jq -er '.run_id' "$base/state/current_runid.json") &&
PATH="$HOME/.foc-devnet/artifacts/foundry/bin:$PATH" \
  FOC_DEVNET_BASEDIR="$base" \
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
