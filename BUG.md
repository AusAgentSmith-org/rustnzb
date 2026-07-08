# rustnzb Bug / Release State

Last updated: 2026-07-08

## Release Gate

The next `rustnzb` release is blocked on test uplift and dependency publication, not on new feature work.

## Completed In This Branch

### Testing uplift

- Added an always-on NZB parser fixture test using `e2e/fixtures/sample.nzb`.
- Converted the main API integration test away from `TestData/` so it now runs with deterministic fixture NZBs.
- Added a full API -> NNTP -> decode -> history integration test using a mock NNTP server and yEnc fixture data.
- Removed hardcoded real-server credentials from `tests/e2e_download_test.rs`; optional live NNTP tests now use `NNTP_PRIMARY_*` environment variables.

### Shared crate contract coverage

- `nzb-nntp 0.2.20` is now published to Forgejo and crates.io.
- The crate now documents that `Downloader` is the simple sequential/failover path.
- Added a regression test proving `Downloader` opens a fresh connection per article attempt.
- README and crate docs now point high-throughput consumers toward pooled/pipelined flows (`ConnectionPool`, `Pipeline`, `nzb-news`).

## Validation Status

### `nzb-nntp`

- `cargo fmt` passed.
- `cargo clippy --all-targets -- -D warnings` passed.
- `cargo test -- --nocapture` passed.

### `rustnzb`

- `cargo fmt` passed.
- `RUSTNZB_SKIP_FRONTEND_BUILD=1 cargo clippy --all-targets -- -D warnings` passed.
- `RUSTNZB_SKIP_FRONTEND_BUILD=1 cargo test -- --nocapture` passed.

Note: the Rust test/build gate currently uses the repo's explicit `RUSTNZB_SKIP_FRONTEND_BUILD=1` path for the test suite. `build.rs` has been hardened to use `npm ci` plus `npm run build` instead of `npx`, so frontend failures are now explicit instead of hanging silently.

### Frontend build diagnosis

- The previous `npx ng build` path could hang waiting on package installation behavior.
- The new build path is deterministic and non-interactive.
- In this environment, `npm ci` is currently failing because requests to `https://registry.npmjs.org/` time out (`ETIMEDOUT`), which points to local/network access to npm rather than a Rust or Angular config bug.

## GitHub / Issue State

- `rustnzb` issue `#11` is the active remaining issue from the current review set.
- The code/docs fix lives in `nzb-nntp`; the issue should be closed only after that crate is published and `rustnzb` is updated to the published version used in the release.
- When closing issues, reference GitHub commit SHAs and GitHub release/tag URLs, not Forgejo-only links.

## Remaining Before Cutting The Next Release

1. Re-run published-crate builds for all release targets.
2. Resolve/link GitHub issues with GitHub commit references.
3. Verify frontend production build/release path in an environment with working npm registry access, or provide a mirrored/internal npm source if local builds must be offline-tolerant.
