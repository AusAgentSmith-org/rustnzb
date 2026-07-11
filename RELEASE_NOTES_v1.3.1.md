# rustnzb v1.3.1

## Summary

rustnzb 1.3.1 is a patch release that closes output file descriptors as soon
as a download finishes and corrects row-divider alignment in the History
table. It also includes release-pipeline reliability improvements made since
1.3.0.

## Fixes

- Completed downloads now unregister their worker-pool job context and clear
  assembler state before post-processing. This prevents output file
  descriptors from accumulating across completed jobs and eventually
  exhausting the process limit.
- History-table action buttons now live in an inner flex container, preserving
  the final cell's native table layout so row dividers remain aligned through
  the last column.
- Shared frontend build artefacts are serialized in CI to prevent concurrent
  release jobs from overwriting one another.
- Release recovery now uses immutable artefacts and no longer retains the
  temporary recovery path used for the completed 1.3.0 publication.

## Validation

- Regression coverage verifies that repeated completed jobs release all file
  descriptors opened beneath their work directories.
- The frontend component suite covers the History view and passes with the
  corrected table structure.

## Breaking changes and upgrade notes

- There are no intentional HTTP API, configuration, or SQLite schema changes.
- Existing container mounts and configuration remain compatible. Operators
  should upgrade normally by pulling `v1.3.1` or `latest` after publication.

## Bundled shared-crate versions

- `nzb-core 0.2.16`
- `nzb-decode 0.1.2`
- `nzb-dispatch 0.2.6`
- `nzb-news 0.1.12`
- `nzb-nntp 0.2.22`
- `nzb-postproc 0.2.6`
- `nzb-web 0.4.20`

## Downloads

- Linux x86_64: `rustnzb-v1.3.1-linux-x86_64.tar.gz`
- Linux aarch64: `rustnzb-v1.3.1-linux-aarch64.tar.gz`
- Windows x86_64 installer: `rustnzb-v1.3.1-windows-x86_64-setup.exe`
- Debian/Ubuntu amd64: `rustnzb-v1.3.1-amd64.deb`
- Debian/Ubuntu arm64: `rustnzb-v1.3.1-arm64.deb`
- Checksums: `SHA256SUMS-v1.3.1.txt`
- Docker: `ghcr.io/ausagentsmith-org/rustnzb:v1.3.1`

All downloadable files are attached to both the Forgejo and GitHub releases
and are also published at `https://dl.rustnzb.dev/v1.3.1/`.
