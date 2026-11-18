# CI Dependency Profiles

[`dependency-profiles.json`](dependency-profiles.json) declares the profiles
accepted by the [dependency resolver](../scripts/resolve-ci-dependencies.py).

## Profiles

The manifest's top-level `profiles` object defines which names the resolver
accepts. The [PR workflow](../.github/workflows/ci_pull_request.yml) uses
`default`. The [nightly workflow](../.github/workflows/ci_nightly.yml) has an
explicit matrix for `stability`, `frontier`, and selected
`stability-frontier-*` profiles. Adding a profile to the manifest makes it
resolvable but does not schedule it; add a matrix entry if nightly CI should
run it.

For each top-level profile, each component must define the selection that
applies to it. The manifest currently uses the component selections
`default`, `stability`, and `frontier`.

Top-level profile definitions have a `base` component profile and can override
specific components:

```json
{
  "stability-frontier-curio": {
    "base": "stability",
    "components": {
      "curio": "frontier"
    }
  }
}
```

In that example, Curio resolves from its `frontier` selection while every other
component resolves from `stability`. Mixed profile names are not generated
automatically.

## Component Fields

Top-level component fields:

- `repository`: Git repository URL.
- `npm_package`: npm package name, for components resolved through npm metadata
  or pkg.pr.new previews.
- `default`, `stability`, `frontier`: component profile selections.

Profile selections always have a `strategy`. Some strategies require additional
fields.

### `config_default`

Use the compiled `Config::default()` value and pass no runtime override to
`foc-devnet init`.

```json
{
  "strategy": "config_default"
}
```

### `git_commit`

Use an exact Git commit SHA.

```json
{
  "strategy": "git_commit",
  "commit": "fadc836e65804311aca3bd2276861acabe42313f"
}
```

### `git_branch`

Resolve a branch head to an immutable commit SHA before the run starts.

```json
{
  "strategy": "git_branch",
  "branch": "master"
}
```

The resolved metadata records both the branch name and the exact commit.

### `git_tag`

Resolve a Git tag to an immutable commit SHA. `tag` can be an exact tag:

```json
{
  "strategy": "git_tag",
  "tag": "v1.2.3"
}
```

`tag` can also be a pattern. Pattern selections choose the latest matching tag:

```json
{
  "strategy": "git_tag",
  "tag": "v*"
}
```

By default, pattern selections exclude prerelease tags such as `-rc`, `-alpha`,
`-beta`, and development tags. Set `include_prereleases` to include them:

```json
{
  "strategy": "git_tag",
  "tag": "v*",
  "include_prereleases": true
}
```

### `git_submodule`

Resolve a git submodule gitlink from a tag or tag pattern in another repository.

```json
{
  "strategy": "git_submodule",
  "repository": "https://github.com/FilOzone/filecoin-services.git",
  "tag": "v*",
  "path": "service_contracts/lib/pdp"
}
```

The resolver first resolves `repository` and `tag` with the same rules as
`git_tag`, then reads `path` from that tree and records the submodule gitlink SHA
as the selected component commit. PDP's `stability` selection uses this
strategy even in mixed profiles that override filecoin-services itself.

### `npm_version`

Resolve an npm version, range, or dist-tag to a concrete package version.

```json
{
  "strategy": "npm_version",
  "version": "1.0.1"
}
```

The `version` field can also be an npm dist-tag:

```json
{
  "strategy": "npm_version",
  "version": "latest"
}
```

The resolver records the concrete package version selected at resolution time
and npm `gitHead` when available.

For `synapse-sdk`, npm resolution also records exact compatible versions of
`@filoz/synapse-core` and the `viem` peer dependency. The scenario installs that
set into a temporary consumer project.

### `pkg_pr_new`

Resolve a branch head to an immutable commit, then install the packages built
for that commit by pkg.pr.new:

```json
{
  "strategy": "pkg_pr_new",
  "branch": "master"
}
```

This strategy is used for Synapse frontier runs. The resolver records the exact
commit and constructs commit-pinned preview URLs for `@filoz/synapse-sdk` and
`@filoz/synapse-core`. The scenario installs those built packages in the same
temporary npm consumer used for released versions.

## Overrides

Some profile selections can include an optional `overrides` object. Each entry
maps a package name to a `version` and a `reason` explaining why the override
exists:

```json
{
  "strategy": "npm_version",
  "version": "1.1.1",
  "overrides": {
    "nanoid": {
      "version": "3.3.13",
      "reason": "nanoid 5.x is ESM-only and breaks the CJS build"
    }
  }
}
```

Overrides are explicit profile policy. Both `version` and `reason` are required
non-empty strings; the resolver rejects any override missing either field, so an
override cannot be added without documenting why. The reason is logged when the
override is applied. The resolver does not infer overrides from package metadata.

Overrides are currently allowed only for npm-installed `synapse-sdk` and
`filecoin-pin` selections. They are written to the temporary consumer
`package.json`.

## Current Boundary

`resolve-ci-dependencies.py` resolves metadata. It does **not** install
components.

The [reusable CI workflow](../.github/workflows/ci_run.yml) passes the
resolved source overrides to `foc-devnet init` for Lotus, Curio,
filecoin-services, and optionally PDP. The
[Synapse runtime](../scenarios/synapse_runtime.py) and
[filecoin-pin scenario](../scenarios/test_multi_copy_upload.py) consume the
metadata when installing their own dependencies.

### Resolver Commands

From the repository root, with Python 3, Git, and npm available:

```sh
metadata="$(mktemp)"
python3 scripts/resolve-ci-dependencies.py resolve \
  --profile stability --output "$metadata"
```

To verify a CI checkout, run the command below **after** `foc-devnet init`
has prepared the repositories for the resolved profile. This checks the Git
commits against the metadata and writes the verified commits back to it.

```sh
python3 scripts/resolve-ci-dependencies.py verify --metadata "$metadata"
```

By default, `verify` looks under `~/.foc-devnet/code`; pass `--code-dir` if
the checkouts are elsewhere. The reusable workflow shows how to use
`--github-output` and `--github-env` to pass the resolved settings to CI steps.

## Local Checks

Install Rust with `rustfmt` and `clippy`, Python 3, and Black (for example,
with `pipx install black`). The lint script skips checks whose tools are
missing. From the repository root:

```sh
FIX=0 ./scripts/lint.sh                  # check only, as in PR CI
cargo test --all-targets --all-features
```

Run `./scripts/lint.sh` without `FIX=0` to apply formatting fixes. To run the
dependency resolver unit tests on their own:

```sh
python3 -m unittest discover -s scripts/tests -p 'test_*.py'
```

Optionally install the pre-commit hook with
`./scripts/install_precommit_hooks.sh`. It runs the lint script in auto-fix
mode by default, so review and stage any formatting fixes separately. The
installer leaves an existing hook in place unless you pass `-f`.
