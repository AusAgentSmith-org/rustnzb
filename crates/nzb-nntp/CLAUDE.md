# nzb-nntp

Async NNTP client library with TLS, pipelining, connection pooling, and multi-server support.

**Version:** 0.1.0 | **Edition:** Rust 2024 | **License:** MIT

## This Is a Shared Library

This crate is the **foundational dependency** in the NZB ecosystem. Breaking changes here affect nearly every app.

### Consumed By

| App | Via | Tag |
|-----|-----|-----|
| rustnzbd | git | v0.1.0 |
| Arz | git | v0.1.0 |
| rustnzbindxer | git | v0.1.0 |
| rustNewsreader | git | latest |
| NGMS | vendored (path) | — |
| nzb-core (lib) | git | v0.1.0 |

### Depended On By (libs)

- **nzb-core** depends on nzb-nntp (for `ServerConfig`, NNTP types)
- **nzb-postproc** depends on nzb-core which depends on nzb-nntp

## Public API

```rust
pub mod config;       // ServerConfig, Article, ListActiveEntry
pub mod connection;   // NntpConnection, NntpResponse, GroupResponse, HeaderEntry, XoverEntry
pub mod downloader;   // Downloader, ArticleResult
pub mod error;        // NntpError, NntpResult
pub mod pipeline;     // Pipeline, StatPipeline, StatResult
pub mod pool;         // ConnectionPool
pub mod server;       // ServerState
```

### Key Types

- **`Downloader`** — high-level multi-server article downloader with failover
- **`ConnectionPool`** — manages pooled NNTP connections per server
- **`Pipeline`** — pipelined NNTP command execution (ARTICLE/BODY batch)
- **`StatPipeline`** — pipelined STAT commands for existence checks
- **`NntpConnection`** — single NNTP connection (TLS, auth, commands)
- **`ServerConfig`** — server connection settings (host, port, TLS, connections, speed limit, etc.)

### Architecture

```
Downloader → ServerState + Pool → ConnectionPool → NntpConnection → Transport
```

## Key Dependencies

- tokio, tokio-rustls, rustls (async + TLS)
- governor (rate limiting / bandwidth throttle)
- flate2 (GZIP compression)
- tokio-socks (SOCKS5 proxy)

## NNTP Commands Supported

ARTICLE, BODY, STAT, GROUP, XOVER, XHDR, XPAT, LIST ACTIVE, AUTHINFO, XFEATURE COMPRESS GZIP, QUIT

## Testing

Mock NNTP server available in `testutil` module for integration tests.
