# nzb-core

Shared models, config, NZB parser, and SQLite database for NZB clients.

**Version:** 0.1.1 | **Edition:** Rust 2024 | **License:** MIT

## This Is a Shared Library

### Consumed By

| App | Via | Tag |
|-----|-----|-----|
| rustnzbd | git | v0.1.1 |
| Arz | git | v0.1.0 |
| rustnzbindxer | vendored (path) | — |
| NGMS | vendored (path) | — |
| nzb-postproc (lib) | git | v0.1.1 |

### Depends On

- **nzb-nntp** (git, v0.1.0) — for `ServerConfig` and NNTP types

## Features

- `groups-db` — enables optional `groups_db` module for newsgroup database operations

## Public API

```rust
pub mod config;          // AppConfig, GeneralConfig, ServerConfig, CategoryConfig, OtelConfig, RssFeedConfig
pub mod db;              // Database — SQLite job/article/segment queries
pub mod error;           // NzbError, Result
pub mod models;          // JobStatus, Priority, NzbJob, ServerArticleStats, etc.
pub mod nzb_parser;      // NZB XML file parsing
pub mod sabnzbd_import;  // SABnzbd config import utility

#[cfg(feature = "groups-db")]
pub mod groups_db;       // Newsgroup database operations
```

### Key Types

- **`AppConfig`** — top-level TOML config (general, servers, categories, otel, rss_feeds)
- **`Database`** — SQLite wrapper for job/article/segment CRUD
- **`NzbJob`** — complete download job with stats
- **`JobStatus`** — Queued, Downloading, Paused, Verifying, Repairing, Extracting, PostProcessing, Completed, Failed
- **`Priority`** — Low(0), Normal(1), High(2), Force(3)
- **`GeneralConfig`** — listen_addr, port, api_key, dirs, speed_limit, cache_size, log_level

## Key Dependencies

- rusqlite (bundled SQLite)
- quick-xml (NZB XML parsing)
- chrono, uuid, serde, toml
