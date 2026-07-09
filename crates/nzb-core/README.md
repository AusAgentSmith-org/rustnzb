# nzb-core

Shared models, configuration, NZB parser, and SQLite database for the `nzb-*` usenet crate stack.

## Features

- NZB XML parser (RFC 4155)
- Job and server configuration types
- SQLite database with WAL mode (download queue, history, newsgroup data)
- Shared error types

## Part of the nzb-* stack

| Crate | Role |
|-------|------|
| [nzb-nntp](https://crates.io/crates/nzb-nntp) | Async NNTP client, connection pool |
| **nzb-core** | Shared models, config, SQLite DB (this crate) |
| [nzb-decode](https://crates.io/crates/nzb-decode) | yEnc decode + file assembly |
| [nzb-news](https://crates.io/crates/nzb-news) | NNTP fetch engine |
| [nzb-dispatch](https://crates.io/crates/nzb-dispatch) | Article dispatcher, retry, hopeless tracking |
| [nzb-postproc](https://crates.io/crates/nzb-postproc) | PAR2 repair, archive extraction |
| [nzb-web](https://crates.io/crates/nzb-web) | Queue manager, download orchestration |

## License

MIT
