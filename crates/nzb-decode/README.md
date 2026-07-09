# nzb-decode

yEnc decoding, file assembly, and article cache for NZB download clients.

Part of the `nzb-*` usenet crate stack. Decodes yEnc-encoded NNTP article bodies using SIMD acceleration (via `yenc-simd`), assembles multi-part articles into complete files, and manages an in-memory article cache.

## Part of the nzb-* stack

| Crate | Role |
|-------|------|
| [nzb-nntp](https://crates.io/crates/nzb-nntp) | Async NNTP client, connection pool |
| [nzb-core](https://crates.io/crates/nzb-core) | Shared models, config, SQLite DB |
| **nzb-decode** | yEnc decode + file assembly (this crate) |
| [nzb-news](https://crates.io/crates/nzb-news) | NNTP fetch engine |
| [nzb-dispatch](https://crates.io/crates/nzb-dispatch) | Article dispatcher, retry, hopeless tracking |
| [nzb-postproc](https://crates.io/crates/nzb-postproc) | PAR2 repair, archive extraction |
| [nzb-web](https://crates.io/crates/nzb-web) | Queue manager, download orchestration |

## License

MIT
