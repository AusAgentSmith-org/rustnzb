# Git Migration Plan

## Objective

Move `rustnzb` and its dependent `nzb-*` crates into a single Cargo monorepo so cross-crate development is fast and atomic, while still publishing and maintaining the shared crates externally.

## Summary

This is feasible and likely a better fit for the current development model than continuing with versioned external crates plus local `[patch]` overrides.

The key constraint is that the monorepo must become the single source of truth. External crate repos can still exist, but they should be mirrors or split outputs of the monorepo rather than equally-primary writable repos.

Do not remove crate versions entirely. Keep `package.version` in every publishable crate. What changes is the inner development workflow: local work should use workspace members and paths, not published versions plus dev patches.

Implementation note after the 2026-07-09 cutover:

- the monorepo layout described here is now live in this repo
- one cleanup item remains intentionally deferred: the root `[patch.crates-io]`
  overrides still need to stay in place while the external private
  `nzbdav-*` crates resolve `nzb-*` dependencies by published version
- removing that patch today breaks `cargo build -p rustnzb --release --features
  webdav` by creating duplicate registry and workspace copies of shared crates

## Current State

Today the project uses:

- versioned shared crates in the root `Cargo.toml`
- local `[patch.crates-io]` overrides for `../libs/nzb-*`
- separate crate repos under `~/Working/libs/`

That works, but it creates two modes of development:

1. published crate mode
2. local patched source mode

That split is the source of most of the friction.

## Target State

Recommended repo shape:

```text
rustnzb/
  Cargo.toml
  Cargo.lock
  apps/
    rustnzb/
      Cargo.toml
      src/
      frontend/
      tests/
  crates/
    nzb-core/
    nzb-nntp/
    nzb-decode/
    nzb-news/
    nzb-dispatch/
    nzb-postproc/
    nzb-web/
  crates-private/
    nzbdav-core/
    nzbdav-stream/
    nzbdav-dav/
    nzbdav-pipeline/
  xtask/
  tools/
```

Notes:

- Do not use git submodules for the crate sources.
- Make the crates normal workspace members.
- Keep each crate independently publishable.
- Keep private crates separate if their registry/publication flow differs.

## Working Rules

These rules should be decided before any file moves:

1. The monorepo is the canonical source of truth.
2. Shared crates remain publishable.
3. External crate repos, if retained, are mirrors only.
4. Daily development uses workspace members, not `[patch]`.
5. Releases are explicit events, not something the dev loop constantly depends on.

## Migration Phases

### Phase 1: Freeze the Operating Model

Before changing layout, decide:

- which repos remain after migration
- whether `nzbdav-*` moves in now or later
- whether `desktop` and `benchnzb` should join the main workspace
- which registry remains authoritative for published crates

Output of this phase:

- a short written policy that names the monorepo as canonical
- a list of crates that will move in the first cut

### Phase 2: Reshape the Repository

Convert the current root package into a real multi-package workspace.

Recommended top-level workspace manifest:

```toml
[workspace]
resolver = "3"
members = [
  "apps/rustnzb",
  "crates/nzb-core",
  "crates/nzb-nntp",
  "crates/nzb-decode",
  "crates/nzb-news",
  "crates/nzb-dispatch",
  "crates/nzb-postproc",
  "crates/nzb-web",
  "crates/mock-nntp-server",
]
exclude = ["benchnzb", "desktop"]
```

Suggested file moves:

- move the current app crate into `apps/rustnzb/`
- move `../libs/nzb-*` into `crates/`
- optionally move `../nzbdav-rs/crates/nzbdav-*` into `crates-private/`

Do this in a branch and expect some CI and path churn during the transition.

### Phase 3: Convert Dependencies

Replace version-plus-patch development with workspace dependency resolution.

Workspace-level dependencies should point at local members:

```toml
[workspace.dependencies]
nzb-web = { path = "crates/nzb-web", features = ["groups-db"] }
nzb-nntp = { path = "crates/nzb-nntp" }
nzb-core = { path = "crates/nzb-core", features = ["groups-db"] }
nzb-decode = { path = "crates/nzb-decode" }
nzb-postproc = { path = "crates/nzb-postproc" }
```

Then consumers can use:

```toml
[dependencies]
nzb-web.workspace = true
```

Within publishable shared crates, use either:

```toml
nzb-core = { version = "0.2.16", path = "../nzb-core" }
```

or a release tool that rewrites path-based manifests during release.

The safest practical starting point is `path + version` for internal publishable dependencies.

### Phase 4: Preserve Publishing

Each crate should keep:

- `package.name`
- `package.version`
- `license`
- `repository`
- `readme`
- correct version constraints for internal published dependencies

Do not treat workspace-local development layout as a substitute for publish metadata.

Release workflow should handle:

1. change detection
2. version bumping
3. changelog generation
4. publish ordering by dependency graph
5. tagging

Recommended tooling:

- `release-plz` as the first option
- `cargo-release` if you prefer more manual control

### Phase 5: Split CI Into Dev CI and Release CI

#### Dev CI

Run on normal pushes and PRs:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

This validates the monorepo as one coherent codebase.

#### Release CI

Run only on release branches, tags, or explicit release workflows:

- detect changed crates
- compute required bumps
- publish in topological order
- create crate tags and release notes
- push mirror updates to standalone crate repos if those remain

### Phase 6: External Repo Strategy

If you want to keep standalone crate repos, do not edit them directly after migration.

Two acceptable models:

1. Read-only mirrors of the monorepo crate directories
2. Automated split repos generated from monorepo paths

Recommended approach:

- use `git subtree split` or equivalent path-based export
- push `crates/nzb-core` to the external `nzb-core` repo
- push tags that match published crate versions

This preserves external visibility without reintroducing dual-source-of-truth problems.

### Phase 7: Roll Out Incrementally

Do not move every crate at once unless there is enough time to absorb the breakage.

Recommended order:

1. `nzb-core`
2. `nzb-nntp`
3. `nzb-decode`
4. `nzb-news`
5. `nzb-dispatch`
6. `nzb-postproc`
7. `nzb-web`
8. `nzbdav-*` if desired

Reasoning:

- this broadly follows dependency direction
- it reduces circular-migration risk
- it makes failures easier to isolate

### Phase 8: Clean Up After Cutover

Once the workspace builds cleanly:

- remove `[patch.crates-io]` only after the external `nzbdav-*` crates no
  longer force mixed registry/workspace resolution
- remove docs that assume `../libs/*` development
- update local guidance and CI docs
- simplify Docker and CI builds so they no longer depend on local out-of-repo checkouts

## Publishing Model

The recommended long-term model is:

- local development uses workspace members
- releases happen deliberately
- version bumps happen when stability is ready
- external consumers continue using normal published crates

That means:

- do not bump crate versions for every cross-crate edit during development
- do bump them when cutting a real release
- do keep manifest versions valid at all times

In practice, this moves versioning out of the inner loop without abandoning semver or published crate hygiene.

## Risks

The main risks are not Cargo syntax. They are process and graph issues:

- accidental dependency cycles once all crates are visible together
- feature leakage across crates
- CI getting slower if every path always builds the full workspace
- release ordering bugs
- confusion if standalone repos are still treated as writable

These are manageable, but they should be expected.

## Recommended First Implementation Cut

For this project specifically:

1. Make the current `rustnzb` repo the monorepo root.
2. Move the app into `apps/rustnzb/`.
3. Move `nzb-*` crates from `~/Working/libs/` into `crates/`.
4. Keep `nzbdav-*` out initially unless active co-development makes the split painful.
5. Use `path + version` for internal publishable crate dependencies.
6. Introduce `release-plz` before the first post-migration publish.
7. Convert standalone crate repos into mirrors or path-split outputs.

This gives the team the fast local cycle immediately while keeping the external crate ecosystem intact.

## Concrete Next Steps

1. Draft the final workspace layout and membership list.
2. Decide whether `nzbdav-*` is in scope for the first migration.
3. Create the root workspace manifest and move the current app into `apps/rustnzb/`.
4. Move one low-level crate first and make the workspace compile.
5. Convert remaining crates in dependency order.
6. Add release automation before publishing the first monorepo-managed crate versions.
7. Remove the old patch-based dev flow and update docs.

## Non-Goals

This plan does not assume:

- removal of crate semver
- abandonment of external registries
- submodule-based source management
- dual maintenance of editable standalone repos

## Decision

Recommended decision:

- proceed with a monorepo
- keep versions in publishable crates
- stop using version bumps and out-of-repo patches as part of the normal inner dev loop
- make the monorepo canonical and automate everything downstream from it
