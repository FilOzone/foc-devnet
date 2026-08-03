# CI Dependency Profiles

`dependency-profiles.json` is the central manifest for CI dependency selection.
Its resolver is located in `scripts/resolve-ci-dependencies.py`.

## Profiles

The manifest declares valid profiles in its top-level `profiles` object:

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

Each component must define a selection for every component profile referenced by
the top-level profile definitions. Today those component selections are
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
component resolves from `stability`. A profile is valid only if it is explicitly
declared in `profiles`; for example, `stability-frontier-filecoin-pin` does not
exist unless added there.

## Component Fields

Top-level component fields:

- `repository`: Git repository URL.
- `npm_package`: npm package name, for components that are resolved through npm
  metadata.
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
as the selected component commit. PDP uses this to pin the same bundled PDP
gitlink as the selected filecoin-services stability tag, even in mixed profiles
that override filecoin-services itself.

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

## Overrides

Some profile selections can include an optional `overrides` object. Each entry
maps a package name to a `version` and a `reason` explaining why the override
exists:

```json
{
  "strategy": "git_tag",
  "tag": "synapse-sdk-v1.0.1",
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

`resolve-ci-dependencies.py` resolves metadata. It does **not** install
components.

Installation currently lives in three places (which consume the resolved
metadata):

- `foc-devnet init`: Lotus, Curio, filecoin-services, and optionally PDP.
- `scenarios/synapse.py`: Synapse SDK scenario dependency.
- `scenarios/test_multi_copy_upload.py`: filecoin-pin scenario dependency.

This split is intentional, but it is not necessarily the final shape. Resolution
and installation should remain separate phases; however, installation behavior
could become declarative and centralized.
