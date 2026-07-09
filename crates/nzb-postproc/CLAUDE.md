# nzb-postproc

Post-processing pipeline: PAR2 verify/repair, archive extraction.

**Version:** 0.1.2 | **Edition:** Rust 2024 | **License:** MIT

## This Is a Shared Library

### Consumed By

| App | Via | Tag |
|-----|-----|-----|
| rustnzbd | git | v0.1.2 |
| Arz | git | v0.1.1 |
| NGMS | vendored (path) | — |

### Depends On

- **nzb-core** (git, v0.1.1) — job models and config
- **rust-par2** (crates.io, v0.1) — PAR2 verify/repair engine

## Public API

```rust
pub mod detect;     // File type detection (par2, RAR, 7z, ZIP)
pub mod par2;       // PAR2 operations
pub mod pipeline;   // Main orchestrator
pub mod unpack;     // Archive extraction (RAR, 7z, ZIP)

pub use detect::ArchiveType;
pub use pipeline::{PostProcConfig, PostProcResult, run_pipeline};
```

### Key Types

- **`run_pipeline(job_dir, config) -> PostProcResult`** — main entry point. Runs: Verify -> Repair -> Extract -> Cleanup.
- **`PostProcConfig`** — cleanup_after_extract, output_dir, articles_failed (0 = skip verify optimization)
- **`PostProcResult`** — success, stages: Vec<StageResult>, error
- **`ArchiveType`** — Rar, SevenZip, Zip

### Pipeline Stages

1. **Verify** — PAR2 integrity check (skipped if articles_failed == 0)
2. **Repair** — PAR2 Reed-Solomon repair if damaged
3. **Extract** — unpack RAR/7z/ZIP archives
4. **Cleanup** — remove par2/archive files after successful extraction

### Detection Functions

- `find_par2_files(dir)` — index files first, then volumes
- `find_archives(dir)` — RAR, 7z, ZIP detection
- `find_cleanup_files(dir)` — candidates for removal

## Key Dependencies

- nzb-core, rust-par2
- walkdir (recursive traversal)
- zip (ZIP extraction)
- tokio (spawn_blocking for PAR2 — VerifyResult is !Send)
