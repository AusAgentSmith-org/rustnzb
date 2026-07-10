# rustnzb v1.3.0

## Summary

rustnzb 1.3.0 brings the application and its shared NZB crates into one
reproducible monorepo, substantially improves queue and history workflows, and
replaces the previous best-effort build path with pinned, containerized release
gates. The release includes native server packages for Linux x86_64, Linux
aarch64, and Windows x86_64, plus multi-architecture Docker images.

## Highlights

### Queue, history, and SABnzbd compatibility

- The Downloads view now combines active queue management with recent history,
  clearer connection-pool information, responsive layout modes, and consistent
  pause/resume state.
- Queue entries can be reordered, and queue filters now remain aligned with the
  global pause state.
- Removing a failed or stalled job preserves its history record instead of
  silently losing the failure context.
- Post-processing state is reported through the SABnzbd-compatible API so
  Sonarr, Radarr, and other clients receive an accurate status.
- Duplicate row actions are guarded while requests are pending, and destructive
  actions use a shared confirmation dialog.

### Server and status controls

- NNTP connection timeout is configurable per server from the API and Settings
  UI. Existing configurations continue to use the default when the field is
  absent.
- `/api/status` now reports total disk capacity alongside available space.
- Settings, history, RSS, groups, media, login, and first-run screens now share
  the same icon, dialog, loading, feedback, and design-token behavior.

### Telemetry, WebDAV, and desktop behavior

- OTLP logs and metrics can be enabled and routed independently while retaining
  the shared `OTEL_ENABLED` and endpoint fallbacks.
- Docker and standalone builds report their source ref through
  `RUSTNZB_BUILD_REF`, making the running revision visible at runtime.
- Media Library/WebDAV controls and queue layout behavior have been tightened.
- The Tauri desktop application now starts and uses the same application HTTP
  server as the standalone build, avoiding a divergent desktop-only backend.

### Reproducible build and release pipeline

- The application, frontend, mock NNTP server, and `nzb-*` crates now live in a
  single Cargo workspace under `apps/rustnzb/` and `crates/`.
- CI tasks run in digest-pinned toolchain images through the checked-in
  `./ci/run` interface.
- Release gates cover formatting, compile checks, workspace tests, Clippy,
  frontend tests and audit, desktop compilation, and the complete 85-test
  Playwright suite.
- The release matrix cross-builds Linux x86_64, Linux aarch64, and Windows
  x86_64, packages both Debian architectures and a Windows installer, and
  verifies the packages before publication.
- Docker publishing now builds immutable candidates, smoke-tests the exact
  pulled image (binary, health endpoint, embedded UI, build ref, 7-Zip, and
  graceful shutdown), then promotes that digest to Forgejo and GHCR.
- Tagged releases publish and verify amd64/arm64 manifests. Ordinary `main`
  pushes update `dev`; only a tag advances `latest`.

## Bundled shared-crate versions

- `nzb-core 0.2.16`
- `nzb-decode 0.1.2`
- `nzb-dispatch 0.2.6`
- `nzb-news 0.1.12`
- `nzb-nntp 0.2.22`
- `nzb-postproc 0.2.6`
- `nzb-web 0.4.20`

## Breaking changes and upgrade notes

- There is no intentional breaking HTTP API or SQLite schema change in this
  release, and existing runtime configuration remains valid.
- Source builds must now target the workspace package from the repository root,
  for example `cargo build -p rustnzb --release --features webdav`. The
  application source and configuration example moved to `apps/rustnzb/`, and
  shared crates moved to `crates/`.
- The root `[patch.crates-io]` entries are intentional. Keep them when building
  with WebDAV so external `nzbdav-*` dependencies use the workspace `nzb-*`
  implementations.
- Container deployments should retain the existing `/config`, `/data`, and
  `/downloads` mounts. Back up configuration and data before upgrading as with
  any production release.
- Operators pinning immutable development images may keep their current SHA.
  `v1.3.0` and `latest` refer to the verified multi-architecture release image.

## Downloads

- Linux x86_64: `rustnzb-v1.3.0-linux-x86_64.tar.gz`
- Linux aarch64: `rustnzb-v1.3.0-linux-aarch64.tar.gz`
- Windows x86_64 installer: `rustnzb-v1.3.0-windows-x86_64-setup.exe`
- Debian/Ubuntu amd64: `rustnzb-v1.3.0-amd64.deb`
- Debian/Ubuntu arm64: `rustnzb-v1.3.0-arm64.deb`
- Checksums: `SHA256SUMS-v1.3.0.txt`
- Docker: `ghcr.io/ausagentsmith-org/rustnzb:v1.3.0`

All downloadable files are attached to both the Forgejo and GitHub releases
and are also published at `https://dl.rustnzb.dev/v1.3.0/`.
