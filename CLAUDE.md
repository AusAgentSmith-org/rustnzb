# CLAUDE.md — rustnzb

## Project Overview

**rustnzb** is a high-performance Usenet NZB download client written in Rust. It provides a REST API, embedded web UI, and SABnzbd-compatible API for integration with *arr applications (Sonarr, Radarr, etc.). The project is a single-binary Cargo workspace, deployed as a Docker container.

## Repository Layout

```
rustnzb/
├── src/main.rs                    # Binary entry point (CLI, config, tracing, startup)
├── frontend/                      # Angular 21 SPA (Material, dark theme, tab-based UI)
├── e2e/                           # Playwright E2E tests
├── build.rs                       # Auto-runs ng build during cargo build
├── benchnzb/                      # Benchmark suite: rustnzb vs SABnzbd (excluded from workspace)
├── desktop/                       # Desktop app (excluded from workspace)
├── tests/                         # Integration tests (e2e download, NNTP, post-processing)
├── config.example.toml            # Configuration reference
├── root/                          # s6-overlay service definitions (copied into container)
├── Dockerfile                     # Multi-stage build (rust:1.88-alpine → linuxserver/baseimage-alpine)
├── docker-compose.yml             # Docker Compose example
├── website/                       # Marketing website (rustnzb.dev)
└── .github/workflows/
    └── docker-deploy.yml          # CI/CD: build → smoke test → deploy
```

## Library Dependencies

All nzb-* crates are published to [crates.io](https://crates.io) and resolved automatically via `cargo build`. No special registry configuration is needed.

| Crate | Purpose |
|-------|---------|
| [nzb-web](https://crates.io/crates/nzb-web) | Axum HTTP server, REST API, queue manager, download engine |
| [nzb-nntp](https://crates.io/crates/nzb-nntp) | Async NNTP client, connection pool, pipelined downloader |
| [nzb-core](https://crates.io/crates/nzb-core) | Shared models, config, NZB parser, SQLite database |
| [nzb-decode](https://crates.io/crates/nzb-decode) | yEnc decoder (SIMD via yenc-simd), file assembler |
| [nzb-postproc](https://crates.io/crates/nzb-postproc) | Post-processing: PAR2 verify/repair, RAR/7z/ZIP extraction |
| [rust-par2](https://crates.io/crates/rust-par2) | PAR2 repair (pure Rust, no external binary needed) |
| [yenc-simd](https://crates.io/crates/yenc-simd) | SIMD yEnc decoder |

## Build & Run

### Prerequisites

- Rust toolchain (1.88+, edition 2024)
- Node.js 22+ (for the Angular frontend — built automatically via `build.rs`)
- System tools for post-processing: `unrar` (or `unrar-free`), `7z` (`p7zip-full`)
- PAR2 is handled by the pure-Rust `rust-par2` crate — no system install needed

### Local Development

```bash
# Debug build (also builds the Angular frontend)
cargo build

# Release build
cargo build --release

# Run tests
cargo test --workspace

# Run with a config file
cargo run -- --config config.toml --port 8080

# Smoke test (verify par2, unrar, 7z are available in the environment)
cargo run -- --smoke-test
```

Copy `config.example.toml` to `config.toml` and fill in your Usenet server details before running.

### Docker

```bash
docker build -t rustnzb:local .

docker run -p 9090:9090 \
  -e PUID=1000 -e PGID=1000 \
  -v ./config:/config \
  -v ./data:/data \
  -v /path/to/downloads:/downloads \
  rustnzb:local
```

### Docker Compose

```bash
docker compose up -d
```

## Configuration

Copy `config.example.toml` → `config.toml`. Full reference is in that file.

**Priority order:** CLI args > environment variables > TOML file > defaults

Key environment variables:

| Variable | Purpose |
|----------|---------|
| `RUSTNZB_CONFIG` | Config file path (default: `config.toml`) |
| `RUSTNZB_PORT` | Listen port (default: 9090) |
| `RUSTNZB_DATA_DIR` | Data directory |
| `RUSTNZB_LOG_LEVEL` | Log level: trace/debug/info/warn/error |
| `OTEL_ENABLED` | Enable OpenTelemetry (`true`/`1`) |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | OTLP gRPC endpoint |

## Architecture

```
                      ┌─────────────┐
                      │   main.rs   │  CLI args, config, tracing, startup
                      └──────┬──────┘
                             │
                      ┌──────▼──────┐
                      │   nzb-web   │  Axum server, REST API, SABnzbd compat
                      │             │  QueueManager (state machine + persistence)
                      │             │  DownloadEngine (orchestrates per-job)
                      └──┬───┬───┬──┘
                         │   │   │
              ┌──────────┘   │   └──────────┐
              ▼              ▼              ▼
        ┌──────────┐  ┌──────────┐  ┌────────────┐
        │ nzb-nntp │  │nzb-decode│  │nzb-postproc│
        │ NNTP pool│  │ yEnc+asm │  │ par2/unrar │
        └──────────┘  └──────────┘  └────────────┘
                             │
                      ┌──────▼──────┐
                      │  nzb-core   │  Models, Config, NZB parser, SQLite DB
                      └─────────────┘
```

## Coding Conventions

- **Rust edition 2024**, workspace resolver v3
- **Async everywhere** — all I/O uses Tokio async/await
- **Error handling**: `thiserror` for library error types, `anyhow` for application-level errors
- **Logging**: `tracing` macros (`info!`, `warn!`, `error!`) — never `println!`
- **No system par2 needed**: uses pure-Rust `rust-par2` library
- **TLS**: rustls with ring crypto provider

## Testing

```bash
# All tests
cargo test --workspace

# Integration tests
cargo test --test e2e_download_test
cargo test --test parse_real_nzbs

# With output
cargo test -- --nocapture
```

## Benchmarking

```bash
cd benchnzb

# Quick benchmark (~5 min)
./run.sh --scenarios quick

# Full benchmark (all 9 scenarios)
./run.sh --scenarios full
```
