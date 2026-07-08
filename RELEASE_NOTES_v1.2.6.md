# rustnzb v1.2.6

## Summary

This release pulls back the worthwhile backend and shared-crate fixes from the
`rustnzb-restyle` fork without taking the broader UI restyle or older vendored
patch stack.

## Notable fixes

- Batched newsgroup header mark-read handling:
  - marks selected headers inside a single DB session
  - stops returning partial silent success when a DB write fails
  - reports the actual `"marked"` count
- Queue row action hardening in the web UI:
  - duplicate pause, resume, and delete clicks are ignored while pending
  - queue refresh now happens after both success and failure
  - row actions return clearer feedback
  - invalid progress and ETA values are clamped instead of surfacing broken UI

## Included shared-crate updates

- `nzb-postproc 0.2.6`
  - avoids passing `-p-` to `7z` when no password is configured
  - keeps existing non-interactive `unrar` behavior
- `nzb-web 0.4.16`
  - fixes auto-preempted downloads getting stuck paused after a higher-priority job finishes
  - fixes direct-unpack prompt detection when `unrar` prints a continuation prompt without a trailing newline

## Tests and verification

Verified during this release pass:

- `rustnzbd`
  - frontend unit tests
  - frontend production build
  - backend mark-read regression tests
  - registry-backed clean build without local path patches
- `nzb-postproc`
  - `cargo fmt --check`
  - `cargo clippy --tests -- -D warnings`
  - `cargo test`
- `nzb-web`
  - `cargo fmt --check`
  - `cargo clippy --tests -- -D warnings`
  - `cargo test`

## Upgrade notes

- No config migration is required for this release.
- No breaking API or schema change is included in the uplift set.

## Published crate line

- `nzb-postproc 0.2.6`
- `nzb-web 0.4.16`
