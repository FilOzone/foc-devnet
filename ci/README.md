# CI Dependency Profiles

`dependencies.toml` is the central manifest for CI dependency selection.
Its resolver is located in `scripts/resolve-dependencies.py`.

## Profiles

The manifest declares valid profiles in its top-level `profiles` tables:

- `default`: used by PR CI, a known-working set of client versions.
- `stability`: used by nightly CI to test stable releases.
- `frontier`: used by nightly CI to test branch heads.
- `stability-frontier-lotus`: used by nightly CI to test stable releases
  except Lotus, which is resolved from `frontier`.
- `stability-frontier-curio`: used by nightly CI to test stable releases
  except Curio, which is resolved from `frontier`.
- `stability-frontier-filecoin-services`: used by nightly CI to test stable
  releases except filecoin-services, which is resolved from `frontier`.
- `stability-frontier-pdp`: used by nightly CI to test stable releases except
  PDP, which is resolved from `frontier`.

Each dependency must define a selection for every profile selection referenced
by the top-level profile definitions. Today those selections are `default`,
`stability`, and `frontier`.

Profile definitions have a `base` selection and can override specific
dependencies:

```toml
[profiles.stability-frontier-curio]
base = "stability"

[profiles.stability-frontier-curio.components]
curio = "frontier"
```

In that example, Curio resolves from its `frontier` selection while every other
dependency resolves from `stability`. A profile is valid only if it is explicitly
declared in `profiles`; for example, `stability-frontier-filecoin-pin` does not
exist unless added there.

The root manifest is also embedded by foc-devnet itself. Rust `Config::default()`
reads the runtime dependency locations it needs from the manifest's `default`
selections, so CI and local defaults share one source of truth. Scenario helpers
read their own dependency entries from the same manifest.

## Dependency Fields

Top-level dependency fields:

- `repository`: Git repository URL.
- `npm_package`: npm package name, for dependencies that are resolved through npm
  metadata.
- `default`, `stability`, `frontier`: dependency profile selections.

Dependencies use named TOML tables so each dependency stays easy to review and
edit:

```toml
[dependencies.lotus]
repository = "https://github.com/filecoin-project/lotus.git"

default = { strategy = "git_tag", tag = "v1.36.1" }
stability = { strategy = "git_tag", tag = "v*" }
frontier = { strategy = "git_branch", branch = "master" }
```

Profile selections always have a `strategy`. Some strategies require additional
fields.

### `bundled`

Use a git submodule bundled with a specific dependency selection.

```toml
strategy = "bundled"
bundle = "filecoin-services@stability"
path = "service_contracts/lib/pdp"
```

The `bundle` field uses `<dependency>@<selection>`. When the active bundled
dependency selection matches that reference, the resolver omits an init override
and lets foc-devnet use the bundled submodule. When a mixed profile changes the
bundled dependency away from that selection, the resolver emits an explicit
`gitcommit:...` override for the bundled commit so the run does not test two
moving contract sources by accident.

### `git_commit`

Use an exact Git commit SHA.

```toml
strategy = "git_commit"
commit = "fadc836e65804311aca3bd2276861acabe42313f"
```

### `git_branch`

Resolve a branch head to an immutable commit SHA before the run starts.

```toml
strategy = "git_branch"
branch = "master"
```

The resolved metadata records both the branch name and the exact commit.

### `git_tag`

Resolve a Git tag to an immutable commit SHA. `tag` can be an exact tag:

```toml
strategy = "git_tag"
tag = "v1.2.3"
```

`tag` can also be a pattern. Pattern selections choose the latest matching tag:

```toml
strategy = "git_tag"
tag = "v*"
```

By default, pattern selections exclude prerelease tags such as `-rc`, `-alpha`,
`-beta`, and development tags. Set `include_prereleases` to include them:

```toml
strategy = "git_tag"
tag = "v*"
include_prereleases = true
```

### `npm_version`

Resolve an npm version, range, or dist-tag to a concrete package version.

```toml
strategy = "npm_version"
version = "1.0.1"
```

The `version` field can also be an npm dist-tag:

```toml
strategy = "npm_version"
version = "latest"
```

The resolver records the concrete package version selected at resolution time
and npm `gitHead` when available.

## Overrides

Some profile selections can include an optional `overrides` object. Each entry
maps a package name to a `version` and a `reason` explaining why the override
exists:

```toml
strategy = "git_tag"
tag = "synapse-sdk-v1.0.1"

[dependencies.synapse-sdk.default.overrides.nanoid]
version = "3.3.13"
reason = "nanoid 5.x is ESM-only and breaks the CJS build"
```

Overrides are explicit profile policy. Both `version` and `reason` are required
non-empty strings; the resolver rejects any override missing either field, so an
override cannot be added without documenting why. The reason is logged when the
override is applied. The resolver does not infer overrides from package metadata.

Overrides are currently allowed only for:

- `synapse-sdk`, because scenario setup controls its pnpm install.
- `filecoin-pin` selections using `npm_version`, because those install into a
  temporary npm project controlled by the scenario.

Current consumers:

- `synapse-sdk` writes overrides to the root `pnpm-workspace.yaml` before
  running `pnpm install`.
- npm-installed `filecoin-pin` writes overrides to the temporary npm
  `package.json` used by the scenario.

## Current Boundary

`resolve-dependencies.py` resolves metadata. It does **not** install
dependencies.

Installation currently lives in three places (which consume the resolved
metadata):

- `foc-devnet init`: Lotus, Curio, filecoin-services, and optionally PDP.
- `scenarios/synapse.py`: Synapse SDK scenario dependency.
- `scenarios/test_multi_copy_upload.py`: filecoin-pin scenario dependency.

This split is intentional, but it is not necessarily the final shape. Resolution
and installation should remain separate phases; however, installation behavior
could become declarative and centralized.
