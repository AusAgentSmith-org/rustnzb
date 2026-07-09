# Fork Merge Status

Path note after the 2026-07-09 monorepo migration:

- historical root-level app paths now live under `apps/rustnzb/`
- historical `Active/apps/libs/...` shared-crate paths now live under
  `crates/...` in this repo

This document records what was reviewed from `FutureMan0/rustnzb-restyle`, what was worth pulling back into the `rustnzb` line, what was actually reproduced before being accepted, which shared crates moved, and what consumer verification still matters.

## Reviewed source

- Fork: `https://github.com/FutureMan0/rustnzb-restyle`
- App repo: `rustnzbd`
- Shared crates:
  - `Active/apps/libs/nzb-web`
  - `Active/apps/libs/nzb-postproc`
  - `Active/apps/libs/nzb-core`
  - `Active/apps/libs/nzb-nntp`

## Decision summary

Do not merge the fork wholesale.

Accepted uplift was narrow:

- keep the backend mark-read correctness fix
- keep minor queue UI hardening
- keep shared-crate fixes only where current code was reproduced as broken or the change was low-risk and directly covered by tests
- ignore major GUI restyle work
- ignore fork changes already superseded upstream

## Accepted uplift set

### 1. Batched mark-read fix in `rustnzbd`

Status: implemented and tested

Files:

- [src/group_handlers.rs](/home/sprooty/Working/Active/apps/rustnzbd/src/group_handlers.rs)
- [tests/group_mark_read.rs](/home/sprooty/Working/Active/apps/rustnzbd/tests/group_mark_read.rs)

What changed:

- added `mark_headers_read`
- `h_header_mark_read` now performs one `with_db` call for the full batch
- error handling now stops on the first real failure instead of silently swallowing per-item errors
- returned `"marked"` count now reflects actual successful work

Tests:

- `mark_headers_read_counts_every_success`
- `mark_headers_read_stops_on_first_error`
- `group_mark_read_updates_counts_and_response`

Verification completed:

- `cargo test mark_headers_read`
- `cargo test --test group_mark_read`

### 2. Minor queue UI hardening in `rustnzbd`

Status: implemented and tested

Files:

- [frontend/src/app/features/queue/queue-view.component.ts](/home/sprooty/Working/Active/apps/rustnzbd/frontend/src/app/features/queue/queue-view.component.ts)
- [frontend/src/app/features/queue/queue-view.component.spec.ts](/home/sprooty/Working/Active/apps/rustnzbd/frontend/src/app/features/queue/queue-view.component.spec.ts)
- [frontend/angular.json](/home/sprooty/Working/Active/apps/rustnzbd/frontend/angular.json)

What changed:

- added per-row pending action tracking
- blocked duplicate pause/resume/delete clicks while a request is in flight
- reloaded the queue after both success and failure
- added snackbar feedback for row actions
- hardened invalid metric handling for percent, ETA, remaining, and duration

Implementation note:

- the first uplift draft still allowed duplicate HTTP work because it passed an already-created observable
- the final fix uses `actionFactory: () => Observable<unknown>` so the request is only created after the pending guard is checked

Tests:

- invalid percent clamping
- invalid ETA handling
- duplicate row action ignored while pending
- queue reload after success
- queue reload plus snackbar on failure

Verification completed:

- `./node_modules/.bin/ng test frontend --watch=false`
- `npm run build -- --configuration=production`

### 3. Shared-crate fixes

#### 3a. `nzb-web` queue preemption regression

Status: reproduced first, then fixed

Files:

- `Active/apps/libs/nzb-web/src/queue_manager.rs`
- `Active/apps/libs/nzb-web/tests/harness_preemption.rs`

What was reproduced:

- with `max_active_downloads(1)`, a low-priority downloading job could be preempted by a high-priority job and remain paused forever after the high-priority job finished

Fix applied:

- added `preempted: bool` tracking on `JobState`
- marked auto-preempted jobs explicitly
- added `resume_preempted_jobs()`
- resumed preempted jobs from `start_next_queued()`
- cleared the flag on manual pause/resume and non-preemption pause paths

Test:

- `preempted_job_resumes_after_high_priority_job_finishes`

Verification completed:

- `cargo test preempted_job_resumes_after_high_priority_job_finishes --test harness_preemption -- --nocapture`

#### 3b. `nzb-web` direct-unpack newline-free prompt hang

Status: reproduced first, then fixed

File:

- `Active/apps/libs/nzb-web/src/direct_unpack.rs`

What was reproduced:

- prompt handling used newline-based reads and could hang when `unrar` emitted the next-volume prompt without a trailing newline

Fix applied:

- switched prompt detection to byte-by-byte stdout processing
- checked prompt, success, and error conditions continuously instead of waiting for newline termination

Test:

- `test_unrar_prompt_without_newline_is_detected`

Verification completed:

- `cargo test test_unrar_prompt_without_newline_is_detected --lib -- --nocapture`

#### 3c. `nzb-postproc` 7z no-password argument handling

Status: fixed conservatively with unit coverage

File:

- `Active/apps/libs/nzb-postproc/src/unpack.rs`

What changed:

- split extractor argument construction into helper functions
- kept `-p-` for `unrar` no-password mode
- omitted `-p-` for `7z` when no password is configured

Tests:

- `sevenz_password_arg_is_omitted_without_password`
- `rar_extract_args_keep_dash_password_only_for_unrar`
- `sevenz_extract_args_do_not_include_dash_password_without_password`

Verification completed:

- `cargo test unpack -- --nocapture`

## Shared crate release status

### `nzb-postproc`

- commit: `d00c73a`
- version: `0.2.6`
- published to Forgejo cargo registry
- mirrored to GitHub `main`

Full crate verification completed:

- `cargo fmt --check`
- `cargo clippy --tests -- -D warnings`
- `cargo test`

### `nzb-web`

- commit: `552ec5c`
- branch: `release/0.4.0`
- version: `0.4.16`
- dependency bump: `nzb-postproc 0.2.5 -> 0.2.6`
- published to Forgejo cargo registry
- mirrored to GitHub `release/0.4.0`

Full crate verification completed:

- `cargo fmt --check`
- `cargo clippy --tests --config 'patch.crates-io.nzb-postproc.path=\"../nzb-postproc\"' -- -D warnings`
- `cargo test --config 'patch.crates-io.nzb-postproc.path=\"../nzb-postproc\"'`

## App integration status

Local working-tree uplift commit:

- `1576386` `Uplift forkmerge fixes and bump shared crate versions`

Equivalent clean-history pushes already made outside this dirty worktree:

- Forgejo `main`: `5a5cf38`
- GitHub `main`: `56f2477`

Dependency line used by the uplift:

- `nzb-web = 0.4.16`
- `nzb-postproc = 0.2.6`

Clean verification already completed in a patch-free worktree against published registry crates:

- `cargo fmt --check`
- `cargo clippy --tests -- -D warnings`
- `cargo build`

## Consumer matrix

This is the actionable consumer list for any future pull-in or regression work.

### 1. `rustnzbd`

Status: primary consumer, fully exercised

Tests to keep:

- backend mark-read unit and integration tests
- frontend queue action and metric hardening tests
- production Angular build
- clean registry-backed `cargo build`

### 2. `Active/apps/nzbservice/gui`

Status: buildable, but not on the affected `nzb-web` line

Observed result:

- `cargo check` passed after restoring expected local `libs/` symlink layout and configuring Forgejo Cargo auth
- the app still resolved `nzb-web v0.1.10`

Implication:

- this consumer did not exercise the `nzb-web 0.4.x` queue or direct-unpack fixes
- no forkmerge follow-up is needed here unless this consumer is explicitly upgraded to the newer shared-crate line

Relevant future tests if it is upgraded:

- queue row action dedupe and refresh behavior
- any direct-unpack or queue-priority workflows exposed through its UI

### 3. `Active/apps/nzbservice/client`

Status: currently incompatible for reasons outside the forkmerge delta

Observed failure:

- `error[E0639]: cannot create non-exhaustive struct using struct expression`

Implication:

- this consumer directly constructs newer `nzb_core::ServerConfig`
- this is consumer drift against the current shared-crate heads, not a forkmerge regression

Relevant future tests only if this consumer is upgraded:

- compile-only coverage for config construction paths
- any API calls relying on queue or group-header behavior

### 4. `myotherrepos/StackArr`

Status: not currently buildable

Observed blocker:

- workspace manifest references missing member `crates/stackarr-postgres`

Implication:

- Cargo fails before dependency resolution
- no useful forkmerge verification can be done until that checkout is repaired

Relevant future tests only after the repo is restored:

- compile check against current `nzb-*` dependency line
- any integration tests touching queue management or direct unpack flows

## Reproduction-first rule

For shared-crate changes, use this order:

1. attempt to reproduce the behavior on current head
2. keep the change only if the bug is real or the fix is narrowly scoped and fully covered by tests
3. add regression tests before publishing the crate

This rule was applied to:

- queue preemption in `nzb-web`
- newline-free `unrar` prompt handling in `nzb-web`

The `7z` password-flag adjustment in `nzb-postproc` was kept as a conservative low-surface fix with direct unit coverage.

## Explicitly rejected fork items

Do not pull these in from the restyle fork as part of this uplift:

- major GUI restyle work
- SSRF protection changes already superseded upstream
- parallel server-health changes already superseded upstream
- any broad visual or product-direction changes not tied to a reproduced bug

## Release status

Release work is complete from the clean release worktree used for publishing:

- `rustnzb` version bumped to `1.2.6`
- Forgejo release commit on `main`: `2b433a1` `release: v1.2.6`
- Forgejo tag `v1.2.6` verified on commit `2b433a13f83ce06b9a52b625e4f6da4ff4792c68`
- GitHub mirror tag `v1.2.6` verified on commit `91c5ef4b0f6e2a880b06fa7ea6c712f23353c58c`
- release notes from `RELEASE_NOTES_v1.2.6.md` were used for the published releases

Published release objects verified:

- Forgejo release `indexarr/rustnzb` id `479`, published, non-draft, non-prerelease
- GitHub release `AusAgentSmith-org/rustnzb` id `350879579`, published, non-draft, non-prerelease
- both releases carry the same five assets:
  - `SHA256SUMS-v1.2.6.txt`
  - `rustnzb-v1.2.6-amd64.deb`
  - `rustnzb-v1.2.6-linux-aarch64.tar.gz`
  - `rustnzb-v1.2.6-linux-x86_64.tar.gz`
  - `rustnzb-v1.2.6-windows-x86_64.zip`

Important note:

- Forgejo and GitHub do not share the same release commit hash for `v1.2.6`; the source-of-truth Forgejo tag points at the private repo history, while the GitHub tag points at the public mirror history

## Net result

The worthwhile uplift from the fork was real, but narrow:

- one backend handler correctness fix
- one small queue UI hardening set
- one reproduced queue preemption fix in `nzb-web`
- one reproduced direct-unpack prompt handling fix in `nzb-web`
- one conservative extractor-argument fix in `nzb-postproc`

Everything else should be treated as either superseded, out of scope, or a separate product decision.
