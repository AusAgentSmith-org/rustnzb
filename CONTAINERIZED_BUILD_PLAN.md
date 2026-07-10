# Containerized Build and Woodpecker Convergence Plan

Status: Proposed

Date: 2026-07-10

Scope: Local development builds, Woodpecker quality gates, release builds, and
container publication for `indexarr/rustnzb`

## Decision

Adopt a container-first build flow in which local development and Woodpecker
use the same versioned toolchain images and the same checked-in task scripts.
Build the production image through one canonical multi-stage Dockerfile, test
the exact candidate image, and promote that image without rebuilding it.

The objective is not to force every job into one large container. Rust checks,
Playwright, desktop/GTK, cross-platform packaging, and the final runtime have
different requirements. The reproducibility guarantee is that a given task
uses the same image digest, command, inputs, and cache policy locally and in
Woodpecker.

## Why This Is Needed

The current pipeline is containerized, but the build definition is split
across several paths:

- `.woodpecker.yml` repeats tool installation and Cargo registry setup in most
  Rust steps.
- normal Linux artifacts compile on Debian/Bookworm for a GNU target.
- the production Dockerfile compiles from Bookworm with Zig for musl.
- `Dockerfile.local` compiles natively on Alpine.
- the frontend can be built by a Woodpecker step and then conditionally reused
  by the Docker build through the shared workspace.
- the production image compiles the application again instead of packaging an
  already verified container candidate.
- Forgejo credentials are currently passed to Docker as build arguments and
  written into a builder layer.
- the Buildx plugin's nested Docker storage has already exhausted its
  overlay filesystem during arm64 builds.

These differences make it difficult to reproduce a CI failure locally and
make successful host builds weak evidence for the image that is eventually
published.

## Goals

1. A developer can reproduce each Woodpecker gate with one local command.
2. Local and CI tasks use the same immutable image digest.
3. Task commands live in the repository, not duplicated in pipeline YAML.
4. The production Docker build is independent of untracked or previously
   generated workspace files.
5. No registry token appears in Docker build arguments, image history, cache
   exports, committed files, or logs.
6. The exact image that passes smoke tests is promoted to `dev`, release, and
   `latest` tags without recompilation.
7. Warm builds become faster without making correctness depend on a warm
   cache.
8. Forgejo remains the private source of truth. GHCR remains a secondary copy
   of an image first published and verified through Forgejo.

## Non-Goals

- Replacing Woodpecker, Forgejo, the Forgejo package registry, or sccache.
- Making the Linux container, GNU tarball, Debian package, Windows installer,
  and Tauri desktop application contain one identical binary. They are
  different product formats and targets, but they will use versioned build
  environments and shared task definitions.
- Enabling generic Docker-in-Docker on every step. The Buildx plugin already
  supplies an isolated Docker environment for image builds.
- Moving private source or primary CI ownership to GitHub.
- Changing runtime configuration, deployment topology, or application
  behavior as part of the build migration.

## Target Repository Layout

The exact filenames can be adjusted during implementation, but the ownership
boundaries should remain:

```text
Dockerfile                         canonical production multi-stage build
Dockerfile.ci                      versioned CI toolchain image targets
rust-toolchain.toml                Rust version, components, and targets
ci/
  README.md                        local usage and image update procedure
  images.lock                      approved image names and digests
  run                              local container task wrapper
  verify-image-pins                checks images.lock against Woodpecker YAML
  tasks/
    cargo-auth                     temporary Forgejo Cargo authentication
    fmt
    check
    test
    clippy
    frontend-test
    e2e
    desktop-test
    build-linux
    build-linux-arm64
    build-windows
    smoke-runtime
    package-release
```

`Dockerfile.local` remains during the transition and is deleted only after the
canonical Dockerfile and local wrapper pass the parity gates.

## Target Build Images

Use a small family of related images rather than one universal image:

| Image target | Contents | Tasks |
|---|---|---|
| `core` | Rust 1.88, rustfmt, clippy, Node 22, npm, protoc, pkg-config, Git, sccache, build tools | fmt, check, test, clippy, frontend unit tests, native Linux build |
| `cross` | `core` plus pinned Zig, cargo-zigbuild, cargo-xwin, LLVM/lld, and required Rust targets | arm64, musl, Windows, and release artifacts |
| `e2e` | `core` plus pinned Playwright Chromium and its system dependencies | browser E2E |
| `desktop` | `core` plus WebKitGTK, AppIndicator, librsvg, and patchelf | Tauri desktop tests |

Build and publish each target to the Forgejo registry with both a source SHA
tag and a human-readable toolchain tag. Record the resolved digest in
`ci/images.lock`. Woodpecker and `ci/run` must execute the digest form, for
example:

```text
repo.indexarr.net/indexarr/rustnzb-ci-core@sha256:<digest>
```

Tags are useful for discovery and rollback; digests provide task parity.
Base images inside `Dockerfile.ci` should also be pinned by digest. Versions of
sccache, Zig, cargo-zigbuild, cargo-xwin, npm, Playwright, and installed system
packages must be deliberate and reviewable. Downloaded standalone tools must
have checksums verified during the image build.

The CI image build cannot consume the new digest before that image exists.
Use this bootstrap sequence when `Dockerfile.ci` changes:

1. Build the new CI images with the last known-good CI image or a manually
   authorized Buildx job.
2. Push SHA-tagged candidates to Forgejo.
3. Run a minimal self-test in each candidate.
4. Resolve and record their digests in `ci/images.lock`.
5. Update `.woodpecker.yml` to those digests in the same reviewable commit.
6. Keep the previous digests available for immediate rollback.

## Local Task Contract

The supported developer entry point will be:

```bash
./ci/run <task>
```

Examples:

```bash
./ci/run fmt
./ci/run test
./ci/run clippy
./ci/run e2e
./ci/run build-image
./ci/run smoke-image rustnzb:local
```

The wrapper will:

- resolve the task to the digest in `ci/images.lock`;
- mount the checkout at the same working directory used by Woodpecker;
- pass the current UID/GID where the selected image supports rootless use;
- configure an explicit `CARGO_TARGET_DIR` rather than relying on a host
  `target/` directory;
- provide optional local Cargo/npm cache volumes without mounting the host's
  toolchain;
- forward credentials by variable name or a protected temporary secret file,
  never by embedding their value in the command line;
- support a cold-cache mode for reproduction;
- remove temporary credentials on exit; and
- invoke exactly one checked-in script from `ci/tasks/`.

Woodpecker steps will select the same digest and invoke the same script. The
pipeline may supply CI metadata, secrets, and remote sccache configuration,
but it must not replace the task's build command with inline YAML.

Task scripts must use `set -eu`, avoid `set -x` when credentials are present,
and print tool versions at startup. A task must fail if a required tool is not
already in its image; it must not silently install packages at runtime.

## Workstream 1: Versioned Build Images

### Changes

1. Add `rust-toolchain.toml` pinned to Rust 1.88 with rustfmt and clippy.
2. Add `Dockerfile.ci` with the `core`, `cross`, `e2e`, and `desktop` targets.
3. Pin base image digests and all standalone tool versions.
4. Add image self-tests that print versions and compile a minimal Rust crate.
5. Publish CI images to Forgejo under SHA and toolchain tags.
6. Add `ci/images.lock` and a verification task that detects drift between the
   lock file, local wrapper, and `.woodpecker.yml`.
7. Document the bootstrap/update/rollback procedure in `ci/README.md`.

### Acceptance Criteria

- Each image builds from a clean checkout without local toolchain mounts.
- Each image self-test passes on its intended architecture.
- The approved digest is used by both local tasks and Woodpecker.
- Re-running a task with the previous digest remains possible.
- No build image contains a Forgejo, GitHub, GHCR, or Woodpecker credential.

### Rollback

Restore the prior entries in `ci/images.lock` and `.woodpecker.yml`. Do not
delete old CI image manifests until at least two subsequent toolchain versions
have completed successful main and tag pipelines.

## Workstream 2: Shared Checked-In Commands

### Changes

1. Extract the current Woodpecker command bodies into scripts under
   `ci/tasks/` without changing their behavior.
2. Make scripts use repository-root-relative paths so they behave identically
   under local Docker and Woodpecker.
3. Centralize temporary Cargo registry configuration in `ci/tasks/cargo-auth`.
4. Add `ci/run` and document prerequisites: Docker/Buildx plus authenticated
   Infisical access for tasks that enable private WebDAV dependencies.
5. Change Woodpecker steps one at a time to call the scripts.
6. Remove `apt-get`, `apk`, `curl | tar`, `cargo install`, `rustup component
   add`, and Playwright browser installation from ordinary task execution.
7. Separate deterministic tests from live policy checks. In particular,
   `npm audit` depends on the current advisory service and should run as a
   clearly named audit gate rather than making local/CI test parity ambiguous.

### Task Inputs and Outputs

Each task must document:

- required image target;
- required and optional environment variables;
- whether private registry access is needed;
- source inputs;
- output paths;
- supported target architecture; and
- whether network access is required after dependencies are present.

No task may consume another step's untracked output unless that artifact is
declared explicitly. `frontend/dist`, `target`, and package output directories
must not be implicit inputs.

### Acceptance Criteria

- `./ci/run fmt`, `check`, `test`, and `clippy` execute the same scripts as
  their Woodpecker equivalents.
- Tool installation no longer appears in normal Woodpecker steps.
- Running a task after deleting `target`, frontend output, and local caches
  succeeds.
- A deliberately failing local task fails with the same command and useful
  diagnostic output in Woodpecker.

### Rollback

During the transition, preserve the previous inline command in Git history and
convert one gate per commit. Revert only the affected step if a shared script
behaves differently; do not roll back unrelated converted gates.

## Workstream 3: Purpose-Built E2E and Desktop Images

### Changes

1. Build Playwright Chromium and operating-system dependencies into the `e2e`
   image at a version matching `e2e/package-lock.json`.
2. Build WebKitGTK, AppIndicator, librsvg, patchelf, and other Tauri test
   dependencies into the `desktop` image.
3. Keep these layers out of `core` so ordinary Rust gates remain small.
4. Add image-level browser and shared-library self-tests.
5. Run E2E against freshly created test data and explicit ports; do not depend
   on a service or database left by another task.
6. Preserve E2E traces/screenshots and desktop test logs as explicit failure
   artifacts.

### Acceptance Criteria

- E2E and desktop steps perform no operating-system package installation.
- The local and Woodpecker E2E tasks use the same browser build.
- The E2E task can start from an empty output directory and clean up its
  processes and fixtures.
- Failures retain enough artifacts to diagnose without rerunning immediately.

### Rollback

Keep the prior `rust:1.88-bookworm` E2E/desktop definitions available for one
transition cycle. If a purpose-built image is broken, restore its previous
digest without changing the test scripts.

## Workstream 4: Canonical Production Dockerfile and Secret Handling

### Changes

1. Refactor `Dockerfile` into named stages for toolchain, frontend dependencies,
   frontend build, Rust build, runtime, and runtime verification.
2. Always build the frontend from declared source and lock files inside the
   Docker build. Remove conditional reuse of `frontend/dist`.
3. Correct `.dockerignore` so nested `target`, `node_modules`, `.angular`, and
   frontend `dist` directories cannot enter the build context.
4. Align the Rust, Node, Zig, and cargo-zigbuild versions with the CI image
   definitions.
5. Replace `ARG GIT_AUTH_TOKEN` and `ARG PLUGIN_PASSWORD` with BuildKit secret
   mounts. The static Cargo registry URL may be stored in the image; the token
   may exist only for the `RUN` instruction that needs it.
6. Add BuildKit cache mounts for Cargo registry/git data and npm downloads.
7. Build the runtime stage from the verified compiled output and include only
   runtime dependencies.
8. Make the local image command use this Dockerfile and the same target and
   build arguments as Woodpecker.
9. Remove `Dockerfile.local` after parity is demonstrated.

Expected secret interface:

```text
docker buildx build \
  --secret id=forgejo_token,env=GIT_AUTH_TOKEN \
  --target runtime \
  --tag rustnzb:local .
```

The Woodpecker Docker Buildx plugin supports build secrets and named targets;
the implementation must use those settings instead of
`build_args_from_env: PLUGIN_PASSWORD`.

### Acceptance Criteria

- A clean local and Woodpecker build produce the same image digest when all
  declared inputs and platform are identical, or any unavoidable provenance
  metadata difference is understood and documented.
- A stale or malicious local `frontend/dist` cannot change the image.
- Secret scanning and `docker history` reveal no registry token or credential
  file.
- The image builds with a cold BuildKit cache.
- `Dockerfile.local` is no longer referenced before it is deleted.

### Rollback

Keep the last known-good production image digest and Dockerfile commit. Do not
promote a candidate built by the refactored Dockerfile until it passes all
runtime checks. `Dockerfile.local` may be retained for one release cycle but
must be clearly marked deprecated.

## Workstream 5: Build Once, Test Candidate, Promote by Digest

### Main-Branch Flow

```text
quality gates
  -> build Forgejo candidate :<commit-sha>
  -> run runtime smoke test against :<commit-sha>
  -> run HTTP health test against :<commit-sha>
  -> inspect manifest and labels
  -> copy the verified digest to :dev
  -> copy the same digest to GHCR :<commit-sha> and :dev
  -> verify all remote references resolve to the expected digest
```

### Tagged-Release Flow

```text
quality and package gates
  -> build :<tag>-amd64 and :<tag>-arm64 candidates
  -> run per-platform build-time smoke checks
  -> run the full runtime/HTTP smoke test on native amd64
  -> create the multi-architecture :<tag> manifest
  -> verify both manifest platforms
  -> promote the same manifests to :latest
  -> copy :<tag> and :latest to GHCR with skopeo
  -> verify Forgejo and GHCR digests/platforms
  -> publish release metadata and standalone artifacts
```

Promotion must use manifest copy/tag operations such as Skopeo or
`manifest-tool`; it must never invoke another compilation. Mutable tags move
only after verification. The source SHA tag remains immutable.

The runtime gate should include:

- `rustnzb --smoke-test` or an equivalent non-destructive binary check;
- startup with an isolated temporary config/data directory;
- polling `/api/health` or the canonical health endpoint with a timeout;
- verification of the embedded frontend root;
- verification that `7z` and other required runtime tools are available;
- graceful shutdown; and
- confirmation that the reported build reference matches the candidate SHA or
  tag.

Woodpecker must prove that a newly pushed candidate can be pulled from Forgejo,
not merely that it exists in the Buildx daemon. If the LinuxServer/s6
entrypoint conflicts with a Woodpecker step command, specify a test entrypoint
or run an explicit candidate test container. Do not weaken the smoke test to a
manifest inspection only.

### arm64 Constraint

The current runner does not have enough nested Buildx overlay capacity for
some arm64 builds. Implement in this order:

1. Prebuild the cross toolchain in the `cross` image.
2. Enable registry-backed BuildKit caching.
3. Stop compiling the frontend outside and inside the Docker build.
4. Measure Buildx storage before and after each arm64 attempt.
5. Prefer packaging an explicit, container-built arm64 output into a thin
   runtime stage if full multi-stage emulation still exceeds storage.
6. Add a native arm64 runner only if the measured build remains constrained;
   do not introduce generic privileged DinD as the first fix.

Until native arm64 runtime execution is available, require a build-time
`--smoke-test` under the target platform plus manifest verification, and
record native arm64 runtime testing as a remaining release risk.

### Acceptance Criteria

- The SHA candidate is immutable.
- `dev`, release, and `latest` resolve to the already-tested digest.
- A failed smoke test leaves mutable tags unchanged.
- GHCR receives the Forgejo-verified manifest rather than a rebuilt image.
- Both registries are checked after promotion.
- A runtime image reports the expected source SHA/tag.

### Rollback

Retain the previously verified digest for each mutable tag. Rollback is a
manifest promotion back to that digest, followed by the same registry and
health verification; it is not a rebuild from an old branch.

## Workstream 6: Explicit Caching and Clean-Build Proof

### Cache Design

Use caches only as performance aids:

- retain Redis-backed sccache for Rust compilation;
- use a namespace that includes compiler version, target triple, and relevant
  feature set;
- use Buildx `cache_images` or explicit registry `cache_from`/`cache_to` for
  Docker layers;
- use BuildKit cache mounts for Cargo registry/git and npm download data;
- use isolated local named volumes through `ci/run` rather than the host's
  Cargo/npm directories;
- do not share `target/` as an undeclared artifact between Woodpecker steps;
- do not cache generated credentials, `frontend/dist`, release packages, test
  databases, or runtime data; and
- apply registry retention rules so cache growth cannot exhaust runner or
  registry storage.

### Clean-Build Gate

Add a manual and scheduled reproducibility workflow that:

1. disables sccache reads or uses a fresh namespace;
2. disables BuildKit cache imports;
3. starts without `target`, frontend output, or package artifacts;
4. runs the core test suite and canonical production build;
5. smoke-tests the candidate; and
6. records duration, output digest, image size, and disk high-water mark.

Normal pull requests may use warm caches. Main and release confidence must not
depend solely on warm-cache runs.

### Metrics

Capture before and after the migration:

| Metric | Cold | Warm |
|---|---:|---:|
| CI image pull time | | |
| fmt/check/test/clippy duration | | |
| frontend unit duration | | |
| E2E duration | | |
| amd64 runtime image build | | |
| arm64 runtime image build | | |
| peak Buildx storage | | |
| final image size | | |
| sccache hit rate | | |
| BuildKit cache reuse | | |

### Acceptance Criteria

- A cold build succeeds at least once on the production runner.
- Warm builds show a measurable improvement without changing test outcomes.
- Cache deletion does not require pipeline edits.
- Runner disk usage stays below an agreed safety threshold during arm64 builds.
- Cache retention and cleanup ownership are documented.

### Rollback

Disable cache imports/exports while preserving the build commands. A cache
failure must reduce performance, not block correctness or require switching
back to host toolchains.

## Migration Sequence

Execute the workstreams in this order:

### Phase 0: Baseline

- Record current local and Woodpecker commands, durations, image sizes, cache
  statistics, and Buildx disk failures.
- Capture one successful main pipeline as the behavioral baseline.
- Confirm Forgejo and Woodpecker authentication without printing credentials.
- Verify which pull-request events are trusted to receive private registry
  secrets.

Exit condition: baseline data and current release references are recorded.

### Phase 1: Shared Scripts

- Add `ci/tasks/` and `ci/run`.
- Move one deterministic gate at a time from inline YAML to a shared script.
- Continue using the current upstream images during this phase.

Exit condition: local and Woodpecker commands match even before custom images
are introduced.

### Phase 2: CI Images

- Add and publish `Dockerfile.ci` targets.
- Pin digests in `ci/images.lock` and Woodpecker.
- Remove runtime tool installation from converted steps.

Exit condition: all quality, E2E, and desktop tasks use approved image digests.

### Phase 3: Canonical Docker Build

- Refactor the production Dockerfile.
- remove implicit frontend artifact reuse;
- switch Docker credentials to BuildKit secrets; and
- prove local/Woodpecker parity before deprecating `Dockerfile.local`.

Exit condition: one Dockerfile produces local and CI runtime candidates.

### Phase 4: Candidate Promotion

- Publish immutable SHA candidates first.
- Add runtime and HTTP smoke gates.
- Promote verified digests to mutable Forgejo and GHCR tags.

Exit condition: no mutable tag moves before the exact image passes its gates.

### Phase 5: Caching and arm64

- Add registry BuildKit cache and measure storage.
- Optimize the arm64 path using evidence from the storage measurements.
- Add the cold-build workflow.

Exit condition: cold builds pass, warm builds improve, and arm64 stays within
the runner's storage budget or has a documented native-runner requirement.

### Phase 6: Cleanup

- Delete `Dockerfile.local` and obsolete inline bootstrap commands.
- Remove temporary compatibility steps and unused cache volumes.
- Update `README.md` and `CLAUDE.md` to make `./ci/run` the supported build
  interface.
- Record final metrics and the rollback digests.

Exit condition: documentation and executable configuration describe the same
supported flow, with no legacy build path presented as equivalent.

## Security Requirements

- Never pass registry credentials through Docker `ARG` or include them in an
  image `ENV` instruction.
- Use BuildKit secret mounts for Docker builds and Woodpecker `from_secret`
  values for task containers.
- Do not echo credentials, enable shell tracing around them, or write them to
  the shared checkout.
- Use a temporary container home or credential file with restrictive
  permissions and remove it on exit.
- Ensure pull requests from untrusted forks do not receive private registry,
  GHCR, release, SSH, or deployment credentials.
- CI images contain tools only; they never contain environment-specific
  credentials or application runtime secrets.
- Keep Forgejo as the first image destination. Copy only verified images to
  GHCR.

## Pull Request and Event Policy

| Event | Gates | Publish |
|---|---|---|
| Pull request | fmt, check, test, clippy, frontend, selected E2E, desktop as resources allow | none |
| Push to `main` | all PR gates, full E2E, immutable runtime candidate, smoke test | Forgejo SHA + `dev`, then GHCR copy |
| Tag | all release gates, packages, amd64/arm64 candidates, smoke/manifest verification | Forgejo tag + `latest`, then GHCR copy and releases |
| Manual cold build | clean core gates, image build, smoke, metrics | none unless separately approved |
| CI image update | image build and self-test | Forgejo SHA/toolchain tags; pipeline pin update after verification |

Network-dependent policy checks such as vulnerability/audit feeds should be
named separately from deterministic compilation and tests so a service outage
cannot be mistaken for local/CI toolchain divergence.

## Definition of Done

The migration is complete when all of the following are true:

- every supported Woodpecker gate has a documented `./ci/run` equivalent;
- local and Woodpecker executions use matching immutable build image digests;
- ordinary task steps install no tools at runtime;
- the production Dockerfile ignores prior workspace build output;
- Docker builds use secret mounts and no credential is visible in history or
  exported cache;
- `Dockerfile.local` is removed;
- the production candidate is smoke-tested before mutable tags move;
- promotion copies the tested digest to Forgejo and then GHCR without rebuild;
- both warm and cold CI paths pass;
- the arm64 limitation is either resolved within the storage budget or backed
  by a documented native-runner decision;
- rollback to the previous verified digest is documented and tested; and
- README/project guidance reflects the container-first workflow.

## Reference Documentation

- Woodpecker workflow syntax and step containers:
  <https://woodpecker-ci.org/docs/usage/workflow-syntax>
- Woodpecker Docker Buildx plugin settings:
  <https://woodpecker-ci.org/plugins/docker-buildx>
- Docker build secrets:
  <https://docs.docker.com/build/building/secrets/>
- Docker multi-stage builds:
  <https://docs.docker.com/build/building/multi-stage/>
- Docker cache backends:
  <https://docs.docker.com/build/cache/backends/>
