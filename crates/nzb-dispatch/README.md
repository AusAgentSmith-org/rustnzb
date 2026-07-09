# nzb-dispatch

Article-level dispatcher for the `nzb-*` usenet crate stack.

Manages a per-server worker pool, priority gating, retry logic, and hopeless-article tracking. Sits between the queue manager and the NNTP fetch layer.

## Architecture

```
nzb-web → nzb-dispatch → nzb-news → nzb-nntp → NNTP server
                       → nzb-decode (article decode)
                       → nzb-core   (shared models)
```

## Part of the nzb-* stack

| Crate | Role |
|-------|------|
| [nzb-nntp](https://crates.io/crates/nzb-nntp) | Async NNTP client, connection pool |
| [nzb-core](https://crates.io/crates/nzb-core) | Shared models, config, SQLite DB |
| [nzb-decode](https://crates.io/crates/nzb-decode) | yEnc decode + file assembly |
| [nzb-news](https://crates.io/crates/nzb-news) | NNTP fetch engine |
| **nzb-dispatch** | Article dispatcher, retry, hopeless tracking (this crate) |
| [nzb-postproc](https://crates.io/crates/nzb-postproc) | PAR2 repair, archive extraction |
| [nzb-web](https://crates.io/crates/nzb-web) | Queue manager, download orchestration |

## License

MIT
