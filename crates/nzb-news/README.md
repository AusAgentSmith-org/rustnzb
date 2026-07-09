# nzb-news

Layered NNTP download engine for the `nzb-*` usenet crate stack.

Provides persistent per-connection worker tasks, priority-aware article dispatch with cascade-retry across servers, and pipelined batch fetching. This is a pure fetch layer — no yEnc decode, no file assembly.

## Architecture

```
nzb-dispatch → nzb-news → nzb-nntp → NNTP server
```

`nzb-news` manages the long-lived per-connection workers and retry logic. `nzb-dispatch` sits above it and handles job-level priority gating and hopeless-article tracking.

## Part of the nzb-* stack

| Crate | Role |
|-------|------|
| [nzb-nntp](https://crates.io/crates/nzb-nntp) | Async NNTP client, connection pool |
| [nzb-core](https://crates.io/crates/nzb-core) | Shared models, config, SQLite DB |
| [nzb-decode](https://crates.io/crates/nzb-decode) | yEnc decode + file assembly |
| **nzb-news** | NNTP fetch engine (this crate) |
| [nzb-dispatch](https://crates.io/crates/nzb-dispatch) | Article dispatcher, retry, hopeless tracking |
| [nzb-postproc](https://crates.io/crates/nzb-postproc) | PAR2 repair, archive extraction |
| [nzb-web](https://crates.io/crates/nzb-web) | Queue manager, download orchestration |

## License

MIT
