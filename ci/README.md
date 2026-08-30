# CI Dependency Profiles

`dependencies.toml` is the central manifest for runtime defaults and CI
dependency selection. Its resolver is located in `scripts/resolve-dependencies.py`.

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

Components can define `default`, `stability`, and `frontier` selections. When a
profile selects a component profile that the component does not define, the
resolver uses that component's `default` selection unless the profile explicitly
overrides that component.

Top-level profile definitions have a `base` component profile and can override
specific components:

```toml
[profiles.stability-frontier-curio]
base = "stability"
curio = "frontier"
```

In that example, Curio resolves from its `frontier` selection while every other
component resolves from `stability`. A profile is valid only if it is explicitly
declared in `profiles`; for example, `stability-frontier-filecoin-pin` does not
exist unless added there.

## Component Fields

Top-level component fields:

- `git`: Git repository URL.
- `npm_package`: npm package name, for components that are resolved through npm
  or pkg.pr.new previews.
- `default`, `stability`, `frontier`: component profile selections.

Profile selections are inline TOML tables. The resolver infers the strategy from
the keys present in each selection.

### `bundled`

Use the component bundled by another dependency and pass no runtime override to
`foc-devnet init`. PDP uses this in the `default` profile so the runtime default
continues to use filecoin-services' bundled submodule.

```toml
default = { bundled = true }
```

### `git_commit`

Use an exact Git commit SHA.

```toml
default = { commit = "fadc836e65804311aca3bd2276861acabe42313f" }
```

### `git_branch`

Resolve a branch head to an immutable commit SHA before the run starts.

```toml
frontier = { branch = "master" }
```

The resolved metadata records both the branch name and the exact commit.

### `git_tag`

Resolve a Git tag to an immutable commit SHA. `tag` can be an exact tag:

```toml
default = { tag = "v1.2.3" }
```

`tag` can also be a pattern. Pattern selections choose the latest matching tag:

```toml
stability = { tag_pattern = "v*" }
```

By default, pattern selections exclude prerelease tags such as `-rc`, `-alpha`,
`-beta`, and development tags. Set `include_prereleases` to include them:

```toml
stability = { tag_pattern = "v*", include_prereleases = true }
```

### `git_submodule`

Resolve a git submodule gitlink from a tag or tag pattern in another repository.

```toml
stability = { submodule_git = "https://github.com/FilOzone/filecoin-services.git", tag_pattern = "v*", path = "service_contracts/lib/pdp" }
```

The resolver first resolves `submodule_git` and `tag_pattern` with the same
rules as `git_tag`, then reads `path` from that tree and records the submodule
gitlink SHA as the selected component commit. PDP uses this to pin the same
bundled PDP gitlink as the selected filecoin-services stability tag, even in
mixed profiles that override filecoin-services itself.

### `npm`

Resolve an npm version, range, or dist-tag to a concrete package version.

```toml
default = { npm = "1.0.1" }
```

The `npm` field can also be an npm dist-tag:

```toml
stability = { npm = "latest" }
```

The resolver records the concrete package version selected at resolution time
and npm `gitHead` when available.

For `synapse-sdk`, npm resolution also records exact compatible versions of
`@filoz/synapse-core` and the `viem` peer dependency. The scenario installs that
set into a temporary consumer project.

### `pkg_pr_new`

Resolve a branch head to an immutable commit, then install the packages built
for that commit by pkg.pr.new:

```toml
frontier = { pkg_pr_new = "master" }
```

This strategy is used for Synapse frontier runs. The resolver records the exact
commit and constructs commit-pinned preview URLs for `@filoz/synapse-sdk` and
`@filoz/synapse-core`. The scenario installs those built packages in the same
temporary npm consumer used for released versions.

## Overrides

Some profile selections can include an optional `overrides` object. Each entry
maps a package name to a `version` and a `reason` explaining why the override
exists:

```toml
default = { npm = "1.1.1", overrides = { nanoid = { version = "3.3.13", reason = "nanoid 5.x is ESM-only and breaks the CJS build" } } }
```

Overrides are explicit profile policy. Both `version` and `reason` are required
non-empty strings; the resolver rejects any override missing either field, so an
override cannot be added without documenting why. The reason is logged when the
override is applied. The resolver does not infer overrides from package metadata.

Overrides are currently allowed only for npm-installed `synapse-sdk` and
`filecoin-pin` selections. They are written to the temporary consumer
`package.json`.

Current consumers write npm overrides to the temporary `package.json` used by
their scenario.

## Current Boundary

`resolve-dependencies.py` resolves metadata. It does **not** install
components.

Installation currently lives in three places (which consume the resolved
metadata):

- `foc-devnet init`: Lotus, Curio, filecoin-services, and optionally PDP.
- `scenarios/synapse_runtime.py`: released or preview Synapse packages.
- `scenarios/test_multi_copy_upload.py`: filecoin-pin scenario dependency.

This split is intentional, but it is not necessarily the final shape. Resolution
and installation should remain separate phases; however, installation behavior
could become declarative and centralized.
