# rustnzbd Review Bugs

Path note after the 2026-07-09 monorepo migration:

- app crate paths under the old repo root now live under `apps/rustnzb/`
- shared `nzb-*` crates that previously lived under `../libs/` now live under
  `crates/`
- file references below are historical to the review date; translate them to
  the monorepo layout when applying follow-up fixes

Reviewed: 2026-07-07

Scope:
- `rustnzbd`
- patched local crates under `../libs/`
- NNTP/download path including `nzb-web`, `nzb-dispatch`, `nzb-news`, `nzb-nntp`, `nzb-core`, `nzb-decode`, `nzb-postproc`

## Findings

### 1. High: live server reconfiguration can strand active jobs

Files:
- `../libs/nzb-dispatch/src/news_engine.rs`
- `../libs/nzb-news/src/downloader.rs`

Details:
- `NewsDispatchEngine::reconcile_servers()` tears down the current `nzb-news` downloader and replaces it with a new one.
- The old downloader emits `FetchOutcome::Cancelled` for pending work during shutdown.
- The adapter drops the tag on `Cancelled` but does not resolve the article against `articles_remaining`.

Effect:
- A job can remain stuck in `Downloading` forever after a mid-download server edit/add/remove because its outstanding articles are no longer reachable and are never counted to completion.

Relevant code:
- `../libs/nzb-dispatch/src/news_engine.rs:377`
- `../libs/nzb-dispatch/src/news_engine.rs:493`
- `../libs/nzb-news/src/downloader.rs:996`

### 2. High: URL fetch SSRF protection is vulnerable to DNS rebinding / TOCTOU

Files:
- `src/handlers.rs`

Details:
- `validate_fetch_url()` resolves the hostname and rejects private/reserved addresses.
- `h_queue_add_url()` then gives the original hostname back to reqwest for the real fetch.
- A rebinding hostname can resolve publicly during validation and privately during the actual request.

Effect:
- The guard is not binding the request to the validated address set.

Relevant code:
- `src/handlers.rs:28`
- `src/handlers.rs:470`
- `src/handlers.rs:473`

### 3. Medium-high: ZIP-based NZB ingestion has no decompressed size limit

Files:
- `src/handlers.rs`

Details:
- `.gz` uploads are capped at 100 MB decompressed.
- `.zip` uploads read matching `.nzb` entries fully into memory with no equivalent cap.
- URL-based adds also buffer the full response body first.

Effect:
- A small archive can expand into a very large in-memory allocation and force avoidable memory pressure or OOM.

Relevant code:
- `src/handlers.rs:289`
- `src/handlers.rs:312`
- `src/handlers.rs:486`

### 4. Medium: group header watermark can advance even when DB inserts fail

Files:
- `src/group_handlers.rs`
- `../libs/nzb-core/src/groups_db.rs`

Details:
- Header batch insert failures are collapsed to `0` with `unwrap_or(0)`.
- The group watermark is then advanced unconditionally to `batch_end`.

Effect:
- On a transient SQLite failure, a whole XOVER range can be marked scanned without ever being persisted, and the missing range will not be retried.

Relevant code:
- `src/group_handlers.rs:230`
- `src/group_handlers.rs:232`
- `src/group_handlers.rs:236`
- `../libs/nzb-core/src/groups_db.rs:142`

### 5. Medium: startup-time server sanitization does not affect the live runtime config

Files:
- `src/main.rs`
- `../libs/nzb-web/src/startup.rs`

Details:
- `main()` trims whitespace from server fields after loading config.
- `startup::initialize()` reloads config from disk and builds the runtime from the untrimmed copy.

Effect:
- The hostname/credential whitespace hardening does not apply to the actual process startup path. API-side edits and inline tests do use the sanitizer, but initial boot does not.

Relevant code:
- `src/main.rs:183`
- `src/main.rs:188`
- `../libs/nzb-web/src/startup.rs:52`
- `../libs/nzb-web/src/startup.rs:106`

## GitHub Issues Review

Reviewed against the public GitHub issue tracker on 2026-07-07:
- `#11` https://github.com/AusAgentSmith-org/rustnzb/issues/11
- `#10` https://github.com/AusAgentSmith-org/rustnzb/issues/10
- `#9` https://github.com/AusAgentSmith-org/rustnzb/issues/9

Implementation status on 2026-07-07:
- `rustnzb` repo work is now in flight on Forgejo branch
  `codex/woodpecker-published-crates`, commit `fc08a8d`
  (`fix: use published crates and harden intake paths`).
- That commit addresses findings 2, 3, and 4 plus the doc portion of issue
  `#11`:
  - binds outbound URL fetches to the validated DNS answers
  - adds response/archive size caps for URL and ZIP/GZIP intake
  - stops advancing group header watermarks after failed inserts
  - updates README runtime layering to include `nzb-dispatch` and `nzb-news`
- Validation completed locally against published dependencies only:
  - `cargo fmt --check`
  - `RUSTNZB_SKIP_FRONTEND_BUILD=1 cargo check --locked --all-targets --no-default-features`
  - `RUSTNZB_SKIP_FRONTEND_BUILD=1 cargo check --locked --features webdav`
  - `CI=1 RUSTNZB_SKIP_FRONTEND_BUILD=1 cargo test --locked --no-default-features`
  - `RUSTNZB_SKIP_FRONTEND_BUILD=1 cargo clippy --locked --all-targets --no-default-features -- -D warnings`
  - `RUSTNZB_SKIP_FRONTEND_BUILD=1 cargo build --locked --release --features webdav`
- Remaining unresolved items from this review:
  - finding 1 (`nzb-dispatch` / `nzb-news` cancellation accounting)
  - upstream `rust-par2` issues `#3` and `#4`
- Additional shared-crate branches now pushed in this session:
  - `nzb-nntp` `codex/ring-only-tokio-rustls`, commit `45ebc32`
  - `nzb-web` `codex/startup-sanitize-fix`, commit `fed2161`
  - `nzb-postproc` `codex/par2-rename-before-extract`, commit `23a8279`

### Issue #11: docs still describe `nzb-nntp::Downloader` as the app's active download engine

Assessment:
- Valid.
- The published `nzb-nntp` crate still contains the old `Downloader` implementation, but the live `rustnzbd` runtime path does not primarily use that type anymore.
- The app wires downloads through `nzb-dispatch` and `nzb-news`, with `nzb-nntp` used lower down for protocol/pipeline primitives.

Evidence:
- `../libs/nzb-nntp/src/downloader.rs:68`
- `../libs/nzb-dispatch/src/news_engine.rs:748`
- `../libs/nzb-dispatch/src/download_engine.rs:1806`
- `README.md:137`

Proposed fix:
- Update README/release docs to describe the current runtime layering accurately:
  - `rustnzb` app
  - `nzb-web` queue manager / API
  - `nzb-dispatch` scheduling + worker orchestration
  - `nzb-news` downloader handle/runtime
  - `nzb-nntp` protocol client, pool, and pipeline building blocks
- Avoid implying that `nzb-nntp::Downloader` is the production path unless that crate is actually re-promoted.

Planned validation:
- Doc-only review; no code test needed.
- Re-read README and release notes after edits to ensure crate/version/runtime claims match the actual tree.

Current state:
- Addressed in `rustnzb` commit `fc08a8d` via README architecture/runtime wording updates.

### Issue #10: `tokio-rustls` defaults re-enable `aws-lc-rs` even though the crate selects `ring`

Assessment:
- Valid.
- `../libs/nzb-nntp/Cargo.toml` currently declares `tokio-rustls = "0.26"`, which enables the crate's default features.
- `tokio-rustls 0.26.4` defaults include `aws_lc_rs`, so the dependency graph currently pulls both `ring` and `aws-lc-rs`.

Evidence:
- `../libs/nzb-nntp/Cargo.toml:12`
- local registry manifest for `tokio-rustls 0.26.4` shows:
  - `default = ["logging", "tls12", "aws_lc_rs"]`
  - `ring = ["rustls/ring"]`
- `cargo tree -i aws-lc-rs -e features` in `../libs/nzb-nntp` shows `tokio-rustls feature "default"` as the path that reintroduces `aws_lc_rs`

Proposed fix:
- Change `nzb-nntp` to an explicit feature selection, e.g.:
  - `tokio-rustls = { version = "0.26", default-features = false, features = ["ring", "logging", "tls12"] }`
- Audit any sibling manifests that directly depend on `tokio-rustls` and apply the same explicit feature policy where needed.

Planned validation:
- Run `cargo tree -i aws-lc-rs -e features` in `../libs/nzb-nntp` after the manifest change and confirm the `tokio-rustls feature "default"` path is gone.
- Re-run `cargo test` in `../libs/nzb-nntp`.

Current state:
- Implemented in `nzb-nntp` branch `codex/ring-only-tokio-rustls`, commit
  `45ebc32`.
- Validation completed:
  - `cargo tree -i aws-lc-rs -e features` now reports no matching package
  - `cargo test`
  - `cargo clippy -- -D warnings`

### Issue #9: encrypted/password-protected 7z archives fail during post-processing

Assessment:
- Partially valid, but the likely root cause differs from the issue title.
- Password plumbing does exist end to end:
  - NZB metadata parsing: `../libs/nzb-core/src/nzb_parser.rs`
  - SABnzbd/API override handling: `../libs/nzb-web/src/sabnzbd_compat.rs`
  - queue manager handoff: `../libs/nzb-web/src/queue_manager.rs:1581`
  - extractor password flags: `../libs/nzb-postproc/src/unpack.rs`
- The stronger failure mode visible from the code and the issue log is archive detection after obfuscated downloads:
  - PAR2-guided rename only runs on the verify path in `../libs/nzb-postproc/src/pipeline.rs`
  - when `articles_failed == 0`, verify is skipped
  - extraction then scans the directory without the PAR2-guided rename having happened
  - obfuscated archive members can remain under names like `.29`, `.28`, etc., so `find_archives()` sees no `.rar`, `.7z`, or `.7z.001` entry and skips extraction entirely

Evidence:
- password flow:
  - `../libs/nzb-core/src/nzb_parser.rs:232`
  - `../libs/nzb-web/src/sabnzbd_compat.rs:195`
  - `../libs/nzb-web/src/queue_manager.rs:1581`
  - `../libs/nzb-postproc/src/unpack.rs:123`
- rename/extract ordering:
  - `../libs/nzb-postproc/src/pipeline.rs:111`
  - `../libs/nzb-postproc/src/pipeline.rs:126`
  - `../libs/nzb-postproc/src/pipeline.rs:306`
  - `../libs/nzb-postproc/src/pipeline.rs:500`
  - `../libs/nzb-postproc/src/detect.rs:198`

Proposed fix:
- Run PAR2-guided rename even when verify is skipped, or at minimum before the extract stage when PAR2 metadata is present.
- Add a second safeguard in extraction if needed:
  - if no archives are found but PAR2 metadata exists, attempt the PAR2-guided rename and rescan once.
- Keep the existing password plumbing, but add a real regression fixture before concluding that password handling itself is broken.

Planned validation:
- Add a regression test in `nzb-postproc` covering:
  - `articles_failed == 0`
  - PAR2 present
  - obfuscated archive filenames on disk
  - expected PAR2 names correspond to extractable archive volumes
  - extraction stage finds archives after rename instead of returning `No archives found`
- Add a targeted test for password propagation if a reproducible encrypted-7z fixture is available.

Current state:
- Implemented in `nzb-postproc` branch
  `codex/par2-rename-before-extract`, commit `23a8279`.
- The fix now runs a PAR2-guided rename from the index `.par2` before the
  extract stage when verify is skipped.
- Validation completed:
  - `cargo fmt --check`
  - `cargo test`
  - `cargo clippy -- -D warnings`
- Regression coverage now includes a proper on-disk PAR2 fixture:
  - `tests/fixtures/obfuscated_zip_par2/release.par2`
  - `tests/fixtures/obfuscated_zip_par2/release.zip`
  - `test_pipeline_renames_obfuscated_zip_using_real_par2_fixture`

## Upstream Dependency Issues: Rust-PAR2

Reviewed against the public GitHub issue tracker on 2026-07-07:
- `#4` https://github.com/AusAgentSmith-org/Rust-PAR2/issues/4
- `#3` https://github.com/AusAgentSmith-org/Rust-PAR2/issues/3

These are not hypothetical upstream cleanups. `rustnzb` pulls `rust-par2` into
its post-processing path today, so both issues are release-relevant for any
attempt to make GitHub CI and published-crate builds the default path.

Current state on 2026-07-07:
- Still unresolved.
- No `rust-par2` code changes have been landed in this session.

### Issue #4: `verify()` undercounts recovery blocks in larger `.volNNN+MMM.par2` files

Assessment:
- Valid, high impact.
- The report shows `rust-par2 0.1.2` returning `recovery_blocks_available = 4`
  for a volume that was explicitly created with 20 recovery blocks.
- The bad count then flips `repair_possible` to `false` even though
  `par2cmdline-turbo` verifies that the set is repairable.

Why this matters to `rustnzb`:
- `rustnzb` relies on `rust-par2` to decide whether repair is possible before
  or during post-processing.
- A false negative here can cause a good Usenet download to be treated as
  unrecoverable, pushing jobs into failed history even when enough PAR2 data is
  present on disk.
- The issue reproduces on realistic recovery volume sizes, not only toy
  fixtures.

Required follow-up:
- Reproduce this locally against the exact `rust-par2` version pulled by
  `rustnzb`.
- Add a regression test in the `Rust-PAR2` repo that exercises a large
  `.volNNN+MMM.par2` file and asserts the full recovery-block count.
- Do not call the pure-Rust PAR2 path production-ready for `rustnzb` until this
  is fixed or a deliberate fallback to `par2cmdline-turbo` remains in place for
  affected cases.

Planned validation:
- `cargo test` in the `Rust-PAR2` repo with a fixture that crosses the failing
  file-size threshold.
- `cargo test` in `rustnzb` / `nzb-postproc` with a fixture or mock proving
  that a repairable set is not misclassified as impossible.

### Issue #3: `repair()` reconstructs wrong bytes when damage spans more than one file

Assessment:
- Valid, high impact.
- The upstream report shows `verify()` correctly identifying damage and enough
  parity blocks, but `repair()` reconstructs the wrong bytes when corruption is
  split across multiple files in the same PAR2 set.
- The crate's own post-repair verification catches the bad result and returns
  `RepairError::VerifyFailed`, which is safer than silent corruption but still a
  hard functional failure.

Why this matters to `rustnzb`:
- Multi-file damage is normal for real Usenet releases, so this is not a corner
  case.
- `rustnzb` can end up with all required PAR2 data present yet still fail the
  repair stage for otherwise recoverable jobs.
- Combined with issue `#4`, the current `rust-par2 0.1.2` release is not yet a
  dependable replacement for `par2cmdline-turbo` across the full repair path.

Required follow-up:
- Reproduce the cross-file repair failure in the `Rust-PAR2` repo.
- Add a regression test that damages blocks across more than one source file
  and asserts byte-identical repaired output.
- Keep or restore an external PAR2 fallback in `rustnzb` until the upstream fix
  is merged, published, and validated under `rustnzb`'s post-processing tests.

Planned validation:
- `cargo test` in the `Rust-PAR2` repo with a multi-file corruption fixture.
- End-to-end post-processing coverage in `nzb-postproc` proving a recoverable
  multi-file release reaches successful completion instead of `VerifyFailed`.

## Validation Notes

Tests run locally:
- `cargo fmt --all --check`
- `cargo build -p rustnzb --release --features webdav`
- `cargo test --workspace -- --skip frugal_auth_all_endpoints --skip ngd_auth --skip supernews_auth`
- `docker build -t rustnzb:local-validate --build-arg GIT_AUTH_TOKEN=... --build-arg RUSTNZB_BUILD_REF=local-validate .`

Results:
- `cargo build -p rustnzb --release --features webdav` passed after restoring a
  constrained root `[patch.crates-io]` section so the external `nzbdav-*`
  crates use the vendored workspace `nzb-*` crates instead of mixing registry
  and path sources.
- Filtered workspace unit/mock/integration suites passed across the app and
  vendored shared crates.
- Live-provider auth tests still need to stay filtered:
  `crates/nzb-nntp/tests/auth_integration.rs` depends on real provider
  accounts, IP reputation, and current service state.
- Local Docker image validation passed, but Docker warned that
  `GIT_AUTH_TOKEN` / `PLUGIN_PASSWORD` are flowing through build args rather
  than a secret mount.

Release/build policy clarified after review:
- Release-candidate builds must succeed against published registries, not local `[patch.*]` overrides.
- Any publish/push/tag work for this project must use the repository's approved migration identity, not a workstation-specific identity.
- This repo already has both Forgejo and GitHub remotes configured:
  - `origin = https://repo.indexarr.net/indexarr/rustnzb.git`
  - `github = https://github.com/AusAgentSmith-org/rustnzb.git`

## crates.io Publication Check

Checked directly against the crates.io API on 2026-07-07.

| Crate | In this repo / workspace | crates.io max version | Status |
|---|---:|---:|---|
| `nzb-web` | `0.4.20` | `0.4.15` | Workspace is ahead of crates.io |
| `nzb-nntp` | `0.2.22` | `0.2.18` | Workspace is ahead; bumped to align with `nzbdav-*` dependency resolution |
| `nzb-core` | `0.2.16` | `0.2.12` | Workspace is ahead of crates.io |
| `nzb-decode` | `0.1.2` | `0.1.2` | Published, current |
| `nzb-postproc` | `0.2.6` | `0.2.5` | Workspace is ahead of crates.io |
| `nzb-news` | `0.1.12` | `0.1.10` | Workspace is ahead of crates.io |
| `nzb-dispatch` | `0.2.6` | `0.2.4` | Workspace is ahead of crates.io |
| `nzbdav-core` | `0.4.1` from Forgejo | `0.4.0` | App depends on a newer Forgejo-published version |
| `nzbdav-stream` | `0.4.1` from Forgejo | `0.4.0` | App depends on a newer Forgejo-published version |
| `nzbdav-dav` | `0.4.1` from Forgejo | `0.4.0` | App depends on a newer Forgejo-published version |
| `nzbdav-pipeline` | `0.4.1` from Forgejo | `0.4.0` | App depends on a newer Forgejo-published version |
| `rust-par2` | `0.1.2` | `0.1.2` | Published, current |
| `yenc-simd` | `0.1.1` | `0.1.1` | Published, current |

Useful sources:
- `https://crates.io/crates/nzb-web`
- `https://crates.io/crates/nzb-nntp`
- `https://crates.io/crates/nzb-core`
- `https://crates.io/crates/nzb-decode`
- `https://crates.io/crates/nzb-postproc`
- `https://crates.io/crates/nzb-news`
- `https://crates.io/crates/nzb-dispatch`
- `https://crates.io/crates/nzbdav-core`
- `https://crates.io/crates/nzbdav-stream`
- `https://crates.io/crates/nzbdav-dav`
- `https://crates.io/crates/nzbdav-pipeline`
- `https://crates.io/crates/rust-par2`
- `https://crates.io/crates/yenc-simd`
