# rustnzb v1.3.3

## Summary

rustnzb 1.3.3 is a correctness-focused patch release for download completion,
provider-failure classification, PAR2 recovery accounting, and terminal job
handling. It prevents temporary provider or connection failures from being
reported as missing content and ensures incomplete downloads cannot enter
post-processing as successful jobs.

## Fixes

- Article absence is now definitive only after every enabled provider returns
  an explicit NNTP 430 response. Authentication, permission, timeout,
  connection, protocol, and unavailable-provider failures remain transient.
- Stale pipelined NNTP sessions reconnect after idle timeouts instead of
  converting the affected articles into permanent failures.
- Completion and hopeless-download decisions now account for usable PAR2
  recovery blocks, unavailable recovery data, the configured safety reserve,
  and the correct PAR2 set.
- Jobs with failed content and no usable PAR2 data now finish as failures
  instead of skipping verification and continuing through extraction.
- Abort and completion paths have a single terminal owner, wait for in-flight
  work to settle, and prevent post-processing from racing assembler writes.
- Terminal queue removal and history persistence are idempotent, preventing
  duplicate history rows and protecting active post-processing from removal.
- Permission-denied NNTP responses receive the same provider cooldown as
  authentication failures and retain their original failure classification.
- The desktop release lockfile and placeholder frontend handling are refreshed
  for reproducible CI builds.

## Website and demo

- The project website has a new NNTP-transcript visual design and updated
  product-focused documentation.
- The interactive browser demo has richer sample data, a documented standalone
  entry point, and an explicit exit control when served under `/demo`.

## Validation

- Regression coverage exercises explicit provider absence, unavailable and
  permission-denied providers, stale-session recovery, PAR2 capacity and safety
  margins, single-owner aborts, post-processing removal, and idempotent history.
- The locked Rust workspace, WebDAV feature build, strict Clippy checks,
  frontend unit suite, dependency audit, and deterministic Playwright suite
  pass for this release candidate.

## Breaking changes and upgrade notes

- There are no intentional HTTP API, configuration, or SQLite schema changes.
- Existing container mounts and configuration remain compatible. Operators
  should upgrade normally by pulling `v1.3.3` or `latest` after publication.

## Bundled shared-crate versions

- `nzb-core 0.2.16`
- `nzb-decode 0.1.2`
- `nzb-dispatch 0.2.6`
- `nzb-news 0.1.12`
- `nzb-nntp 0.2.22`
- `nzb-postproc 0.2.6`
- `nzb-web 0.4.20`

## Downloads

- Linux x86_64: `rustnzb-v1.3.3-linux-x86_64.tar.gz`
- Linux aarch64: `rustnzb-v1.3.3-linux-aarch64.tar.gz`
- Windows x86_64 installer: `rustnzb-v1.3.3-windows-x86_64-setup.exe`
- Debian/Ubuntu amd64: `rustnzb-v1.3.3-amd64.deb`
- Debian/Ubuntu arm64: `rustnzb-v1.3.3-arm64.deb`
- Checksums: `SHA256SUMS-v1.3.3.txt`
- Docker: `ghcr.io/ausagentsmith-org/rustnzb:v1.3.3`

All downloadable files are attached to both the Forgejo and GitHub releases
and are also published at `https://dl.rustnzb.dev/v1.3.3/`.
