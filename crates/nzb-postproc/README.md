# nzb-postproc

Post-processing pipeline for the `nzb-*` usenet crate stack.

Handles PAR2 verification and repair (via the pure-Rust `rust-par2` crate — no external binary needed) and archive extraction (RAR, ZIP, 7z).

## Features

- PAR2 verify and repair using native Rust (no `par2cmdline` required)
- RAR extraction (`unrar` system binary)
- ZIP and 7z extraction

## Part of the nzb-* stack

| Crate | Role |
|-------|------|
| [nzb-nntp](https://crates.io/crates/nzb-nntp) | Async NNTP client, connection pool |
| [nzb-core](https://crates.io/crates/nzb-core) | Shared models, config, SQLite DB |
| [nzb-decode](https://crates.io/crates/nzb-decode) | yEnc decode + file assembly |
| [nzb-news](https://crates.io/crates/nzb-news) | NNTP fetch engine |
| [nzb-dispatch](https://crates.io/crates/nzb-dispatch) | Article dispatcher, retry, hopeless tracking |
| **nzb-postproc** | PAR2 repair, archive extraction (this crate) |
| [nzb-web](https://crates.io/crates/nzb-web) | Queue manager, download orchestration |

## License

MIT
