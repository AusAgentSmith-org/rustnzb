# rustnzbd Uplift Plan: Add Newsreader & Angular UI

Path note after the 2026-07-09 monorepo migration:

- app crate paths previously written as `src/...`, `tests/...`, or `frontend/...`
  now live under `apps/rustnzb/`
- shared `nzb-*` crate paths previously written as `../libs/<crate>/...` now
  live under `crates/<crate>/...`
- this plan predates the monorepo cutover; keep that translation in mind while
  using the implementation notes below

## Review Blockers Before Further Uplift

The current codebase has a few correctness and hardening issues that should be fixed before expanding the feature surface further.

### Priority blockers from 2026-07-07 review

1. Server reconfiguration can strand active jobs:
   - `../libs/nzb-dispatch/src/news_engine.rs`
   - `../libs/nzb-news/src/downloader.rs`
   - Mid-download server edits can leave jobs stuck in `Downloading` because `Cancelled` work from the old downloader is not resolved against job completion accounting.

2. `add-url` SSRF guard is not binding the actual outbound request:
   - `src/handlers.rs`
   - The hostname is validated first, then reqwest resolves it again later, leaving a DNS rebinding / TOCTOU gap.

3. ZIP-based NZB ingestion has no decompressed size limit:
   - `src/handlers.rs`
   - `.gz` is capped, `.zip` is not. Large archive expansion can still force unbounded allocation.

4. Group header watermark can advance on failed insert:
   - `src/group_handlers.rs`
   - `../libs/nzb-core/src/groups_db.rs`
   - A failed header batch can still move `last_scanned`, causing permanent header loss for that range.

5. Startup config sanitization does not affect the live runtime config:
   - `src/main.rs`
   - `../libs/nzb-web/src/startup.rs`
   - The trim/sanitize pass runs on an early config copy that is discarded before engine startup.

Current implementation state on 2026-07-07:
- Addressed in `rustnzb` branch `codex/woodpecker-published-crates` / commit
  `fc08a8d`:
  - blocker 2 (`add-url` DNS rebinding / TOCTOU)
  - blocker 3 (ZIP/response size caps)
  - blocker 4 (header watermark advancement after failed insert)
- Addressed in shared crates:
  - blocker 5 (`nzb-web`) in branch `codex/startup-sanitize-fix`, commit
    `fed2161`
  - GitHub issue `#10` / `nzb-nntp` TLS feature cleanup in branch
    `codex/ring-only-tokio-rustls`, commit `45ebc32`
- Shared-crate issue now addressed in this session:
  - issue `#9` / `nzb-postproc` in branch
    `codex/par2-rename-before-extract`, commit `23a8279`
- Still pending in shared crates:
  - blocker 1 (`nzb-dispatch` / `nzb-news`)
- Still pending upstream:
  - `rust-par2` issues `#3` and `#4`

Detailed write-up:
- See `BUG.md`

### Crate publication status to keep in mind

Checked against crates.io on 2026-07-07:

- Published and current there:
  - `nzb-decode 0.1.2`
  - `rust-par2 0.1.2`
  - `yenc-simd 0.1.1`
- Workspace is ahead of crates.io:
  - `nzb-web`: workspace `0.4.20`, crates.io `0.4.15`
  - `nzb-nntp`: workspace `0.2.22`, crates.io `0.2.18`
  - `nzb-core`: workspace `0.2.16`, crates.io `0.2.12`
  - `nzb-postproc`: workspace `0.2.6`, crates.io `0.2.5`
  - `nzb-news`: workspace `0.1.12`, crates.io `0.1.10`
  - `nzb-dispatch`: workspace `0.2.6`, crates.io `0.2.4`
- WebDAV crates are published to crates.io at `0.4.0`, but this app depends on `0.4.1` from Forgejo:
  - `nzbdav-core`
  - `nzbdav-stream`
  - `nzbdav-dav`
  - `nzbdav-pipeline`
- The root `[patch.crates-io]` section is intentionally still present after the
  monorepo migration because those external `nzbdav-*` crates otherwise pull a
  second registry copy of shared `nzb-*` dependencies during `--features
  webdav` builds.

### Release constraints

- Local path overrides are for development only. Any release gate for this app or its shared crates must pass using published dependencies from Forgejo/crates.io, not unpublished local checkouts.
- Before any commit/tag/publish/push for release work, set the repo-local approved migration identity so release history is not authored as a workstation-specific user.
- This repo has a Forgejo primary remote and a GitHub mirror remote already configured; release flow must keep both in sync.
- Add a full dependency version review across `rustnzb` and the shared crates
  before calling the release/uplift line complete. That review should cover:
  - pinned versions vs latest published versions
  - duplicated transitive TLS/runtime stacks
  - stale patches or branch-only assumptions
  - whether each crate version consumed by `rustnzb` is actually published and
    buildable without local overrides

### GitHub cutover tasks

Current state checked on 2026-07-07:

- GitHub Actions are enabled for `AusAgentSmith-org/rustnzb`.
- An org-scoped runner, `node-b-rustnzb`, is now online in the default runner
  group with labels `self-hosted`, `Linux`, `X64`, and `rustnzb`.
- `rustnzb` `main` on GitHub now contains a repo-local `CI` workflow and the
  published-crates manifest cleanup needed for clean GitHub checkouts.
- The GitHub-side CI branch line was validated locally with published
  dependencies only:
  - `cargo fmt --check`
  - `cargo clippy --locked --all-targets --no-default-features -- -D warnings`
  - `CI=1 RUSTNZB_SKIP_FRONTEND_BUILD=1 cargo test --locked --no-default-features`
- `build.rs` now supports `RUSTNZB_SKIP_FRONTEND_BUILD=1` so Rust-only CI does
  not block on an Angular production build.
- Existing release/build work still assumes Forgejo-first flows in several
  places and needs to be inverted carefully if GitHub becomes primary.
- Manual `workflow_dispatch` against the new `CI` workflow returned GitHub API
  `422` with `Actions has been disabled for this user`, so repo-level Actions
  are enabled and the runner is healthy, but workflow execution still appears
  blocked for the current PAT-backed user identity.
- Forgejo has not yet been updated to mirror the new GitHub `main` state back
  in the other direction, because `origin/main` and `github/main` diverged
  before this cutover and the secondary-branch policy still needs an explicit
  decision.
- Pivot back to Forgejo/Woodpecker is now in progress because GitHub workflow
  execution remains blocked for the current user identity.
- A Forgejo branch carrying the published-crates and intake-hardening work has
  been pushed as `codex/woodpecker-published-crates` at commit `fc08a8d`.
- That Forgejo branch has already passed the equivalent local build/test gates
  against published dependencies only:
  - `cargo fmt --check`
  - `RUSTNZB_SKIP_FRONTEND_BUILD=1 cargo check --locked --all-targets --no-default-features`
  - `RUSTNZB_SKIP_FRONTEND_BUILD=1 cargo check --locked --features webdav`
  - `CI=1 RUSTNZB_SKIP_FRONTEND_BUILD=1 cargo test --locked --no-default-features`
  - `RUSTNZB_SKIP_FRONTEND_BUILD=1 cargo clippy --locked --all-targets --no-default-features -- -D warnings`
  - `RUSTNZB_SKIP_FRONTEND_BUILD=1 cargo build --locked --release --features webdav`
- Woodpecker pipeline verification for that branch still needs a successful API
  trigger/readback.

Current task status:

1. Completed: deploy an org-scoped GitHub Actions runner on Node B via the
   `ops` repo and Komodo, with narrow labels for `rustnzb`.
2. Completed: land a GitHub Actions workflow in `rustnzb` that builds and
   tests with published crates only, not local path overrides.
3. Completed: push GitHub CI changes with the `AusAgentSmith` git identity.
4. Partial: move GitHub toward the primary build/review surface for `rustnzb`;
   execution is still blocked by the current GitHub user-level Actions error.
5. Pending: keep Forgejo as the deliberate secondary mirror with an explicit
   branch policy instead of an accidental divergence.
6. In progress: remove Dependabot configuration and Dependabot workflows from
   every repo in `AusAgentSmith-org`.
7. Pending: audit org mirror repos and document GitHub-vs-Forgejo ownership per
   repo.

Dependabot cleanup status as of 2026-07-07:

- Removed checked-in Dependabot config from writable repos that still owned it:
  - `rustnzb`
  - `rustTorrent`
- Removed repo-local `dependabot-linear.yml` workflows from writable repos that
  used them:
  - `Indexarr_Website`
  - `NoteTaker`
  - `NoteTaker_Website`
  - `rustTorrent_Website`
  - `-rustnzbd_Website`
  - `RustNZBIndexer`
  - `pipeline-dash`
  - `Newznab-API-Spec-Validator`
- Remaining exception:
  - `Indexarr` still contains `.github/workflows/dependabot-linear.yml`, but
    the repo is archived and GitHub rejects writes with `403 Repository was
    archived so is read-only`.
- GitHub may continue to show generated `dynamic/dependabot/dependabot-updates`
  workflow entries temporarily or due to higher-level policy, even after the
  repo-owned config/workflow files are removed. The checked-in sources under
  repository control have been removed everywhere except the archived repo.

### Upstream dependency blockers for release confidence

- `rust-par2` has two open functional issues on GitHub as of 2026-07-07:
  - `#3` `repair()` fails on cross-file damage
  - `#4` `verify()` undercounts recovery blocks in larger `.volNNN+MMM.par2`
    files
- Because `rustnzb` depends on `rust-par2`, these should be treated as release
  blockers or require an explicit fallback strategy.
- Detailed notes live in `BUG.md`.

## Goal

Transform rustnzbd from a headless NZB downloader into a full **NZBGet competitor with built-in newsgroup browsing**. Two major workstreams:

1. **Add newsreader features** — Browse groups, download headers, search articles, select binaries to download
2. **Replace vanilla JS UI with Angular SPA** — Modern, maintainable, component-based frontend

## Current State

rustnzbd already has:
- Complete NZB download pipeline (queue, yEnc decode, file assembly, PAR2, extract)
- Multi-server NNTP with pooling, pipelining, failover
- SABnzbd API compatibility (Sonarr/Radarr ready)
- RSS feed monitoring with download rules
- Queue management (pause/resume/priority/categories)
- History, logging, JWT auth
- Vanilla JS web UI (3,087-line index.html)
- Tauri desktop wrapper
- 50+ REST API endpoints

The nzb-nntp crate already has GROUP, XOVER, LIST ACTIVE, ARTICLE commands — just not exposed in the web UI.

---

## Phase 1: Backend — Newsreader API Endpoints

Add endpoints for browsing Usenet directly from the app. No schema changes needed for the NNTP operations (they're live queries), but we need tables for subscribed groups and cached headers.

### 1.1 New Database Tables

```sql
-- Subscribed newsgroups
CREATE TABLE groups (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL UNIQUE,
    description TEXT,
    subscribed  INTEGER NOT NULL DEFAULT 0,
    article_count INTEGER NOT NULL DEFAULT 0,
    first_article INTEGER NOT NULL DEFAULT 0,
    last_article  INTEGER NOT NULL DEFAULT 0,
    last_scanned  INTEGER NOT NULL DEFAULT 0,
    last_updated  TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Cached article headers from XOVER
CREATE TABLE headers (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id    INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    article_num INTEGER NOT NULL,
    subject     TEXT NOT NULL,
    author      TEXT NOT NULL,
    date        TEXT NOT NULL,
    message_id  TEXT NOT NULL,
    references_ TEXT NOT NULL DEFAULT '',
    bytes       INTEGER NOT NULL DEFAULT 0,
    lines       INTEGER NOT NULL DEFAULT 0,
    read        INTEGER NOT NULL DEFAULT 0,
    downloaded_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- FTS5 for header search
CREATE VIRTUAL TABLE headers_fts USING fts5(
    subject, author, content='headers', content_rowid='id',
    tokenize='porter unicode61'
);
-- + insert/delete/update triggers to keep FTS in sync
```

### 1.2 New API Endpoints

Add to the Axum router in `crates/nzb-web/src/server.rs`:

```
# Group browsing
GET    /api/groups                    # List groups (filter: subscribed, search)
POST   /api/groups/refresh            # Fetch LIST ACTIVE from server
GET    /api/groups/{id}               # Group details
GET    /api/groups/{id}/status        # Article count, new available, unread
POST   /api/groups/{id}/subscribe     # Subscribe
POST   /api/groups/{id}/unsubscribe   # Unsubscribe

# Header browsing
GET    /api/groups/{id}/headers       # Paginated headers (search via FTS5)
POST   /api/groups/{id}/headers/fetch # Trigger XOVER download (background)
GET    /api/groups/{id}/threads       # Threaded view (grouped by References)
GET    /api/groups/{id}/threads/{root_msg_id}  # Thread detail with depth
POST   /api/groups/{id}/headers/mark-read      # Bulk mark read
POST   /api/groups/{id}/headers/mark-all-read  # Mark all read

# Article reading
GET    /api/articles/{message_id}     # Fetch article from NNTP
GET    /api/articles/{message_id}/body # Body only

# NZB generation from selected headers
POST   /api/groups/{id}/headers/download  # Select headers → create NZB → add to queue
```

The last endpoint is the **key integration point**: user browses headers, selects binary posts, and the backend generates an NZB from those message-IDs and adds it to the download queue. This bridges the newsreader and downloader.

### 1.3 Implementation

All the code for these features already exists in rustNewsreader. Port from:

| Feature | Source (rustNewsreader) | Target (rustnzbd) |
|---------|------------------------|---------------------|
| Group list/subscribe | `nr-core/src/db.rs` (group CRUD) | `nzb-core/src/db.rs` |
| Header fetch + store | `nr-web/src/services/header.rs` | `nzb-web/src/handlers.rs` |
| Thread detection | `nr-core/src/db.rs` (list_threads) | `nzb-core/src/db.rs` |
| FTS5 search | `nr-core/src/db.rs` (FTS5 MATCH) | `nzb-core/src/db.rs` |
| Read/unread tracking | `nr-core/src/db.rs` (read_articles) | `nzb-core/src/db.rs` |
| LIST ACTIVE | `nzb-nntp` (already shared) | Already available |

Note: rustnzbd uses `rusqlite` (not sqlx). The SQL is the same, just different Rust API.

### 1.4 "Download Selected" Flow

This is the killer feature that connects newsreading to downloading:

1. User browses headers in a group
2. Selects articles (e.g., all parts of "Movie.2024.BluRay.1080p")
3. Clicks "Download Selected"
4. Backend:
   - Groups selected message-IDs by filename (parse subject for part numbers)
   - Generates an NZB XML in memory (reuse `nzb_generator.rs` from rustnzbindxer)
   - Calls `queue_manager.add_job()` with the generated NZB
5. Download appears in queue immediately

---

## Phase 2: Angular SPA Frontend

Replace the 3,087-line vanilla JS `index.html` with a proper Angular application.

### 2.1 Project Setup

```
rustnzbd/
  frontend/              # NEW: Angular workspace
    src/app/
      core/
        services/        # API, WebSocket, auth services
        models/          # TypeScript interfaces matching Rust models
        guards/          # Auth guard
      features/
        queue/           # Download queue (main view)
        history/         # Completed downloads
        groups/          # Newsgroup browser (NEW)
        headers/         # Article headers + threaded view (NEW)
        rss/             # RSS feeds + rules
        settings/        # Server, category, general config
        logs/            # Log viewer
      shared/
        components/      # Progress bar, speed graph, toolbar
```

### 2.2 Views (NZBGet-style layout)

**Tab-based navigation** matching NZBGet's proven UX:

| Tab | Content |
|-----|---------|
| **Queue** | Active downloads with progress bars, speed, ETA, pause/resume |
| **Groups** | Subscribed groups sidebar + header list + article preview (3-panel) |
| **History** | Completed/failed downloads, retry button |
| **RSS** | Feed items, download rules |
| **Settings** | Servers, categories, general config |
| **Logs** | Real-time log stream |

The **Groups** tab is the new newsreader view. Everything else maps 1:1 to existing API endpoints.

### 2.3 Queue View (primary)

```
┌─────────────────────────────────────────────────────────┐
│ [+Add NZB] [Pause All] [Resume]       ▼ 45.2 MB/s      │
├─────────────────────────────────────────────────────────┤
│ ▼ Movie.2024.1080p            DOWNLOADING  [████░░] 62% │
│   4.2 GB · ETA 3:42 · High · Movies                     │
│                                                          │
│ ● TV.Show.S01E01              UNPACKING    [██████] 100%│
│   1.1 GB · Completed · TV                               │
│                                                          │
│ ○ Software.Package            QUEUED                     │
│   650 MB · Normal · Software                             │
├─────────────────────────────────────────────────────────┤
│ Queue │ Groups │ History │ RSS │ Settings │ Logs         │
└─────────────────────────────────────────────────────────┘
```

### 2.4 Groups View (newsreader)

```
┌──────────────┬──────────────────────────────────────────┐
│ Groups       │ alt.binaries.multimedia                   │
│              │ ┌──────────────────────────────────────┐  │
│ alt.bin.*  9 │ │ Subject           Author    Size Date│  │
│ comp.*     2 │ │ □ Movie.2024 [1/50] post@  4.2G 3/28│  │
│              │ │ □ Movie.2024 [2/50] post@  4.2G 3/28│  │
│              │ │ ☑ TV.Show.S01 [1/20] up@   1.1G 3/27│  │
│              │ │ ...                                  │  │
│              │ ├──────────────────────────────────────┤  │
│              │ │ [Download Selected] [Mark All Read]  │  │
│              │ │ Article preview pane                 │  │
│              │ └──────────────────────────────────────┘  │
└──────────────┴──────────────────────────────────────────┘
```

### 2.5 Port from rustNewsreader

These Angular components already exist and can be ported:

| Component | Source (rustNewsreader) | Adaptation |
|-----------|------------------------|------------|
| Group sidebar | `newsreader-view.component.ts` (groups panel) | Extract, style for tab |
| Header list (flat) | Same component (header-table section) | Add checkbox selection |
| Thread view | Same component (threaded section) | Port as-is |
| Article preview | Same component (article panel) | Port as-is |
| Compose dialog | `compose-dialog.component.ts` | Port as-is |
| Server form | `server-form-dialog.component.ts` | Port as-is |
| API service | `api.service.ts` | Add auth headers |
| WebSocket service | `websocket.service.ts` | Port as-is |
| Models | `*.model.ts` | Extend with download models |

**New components to build:**

| Component | Purpose |
|-----------|---------|
| Queue list | Download queue with progress bars |
| History list | Completed downloads |
| RSS feed manager | Feed CRUD + item browser |
| Settings page | Tabbed config editor |
| Log viewer | Real-time log stream |
| Speed graph | Canvas-based speed chart |
| Auth screens | Login + setup |

### 2.6 Build Integration

Same approach as rustNewsreader:
- `build.rs` runs `ng build --configuration=production`
- `rust-embed` embeds `frontend/dist/` into binary
- Single binary serves both API and SPA
- Dev mode: `ng serve` with proxy to `:9090`

---

## Phase 3: Polish & Differentiation

### 3.1 Features NZBGet doesn't have
- **Built-in newsgroup browser** — Browse, search, and download directly from groups
- **Article preview** — Read text articles inline
- **Header threading** — Conversation view for group discussions
- **FTS5 search** — Fast full-text search across cached headers
- **Generate NZB from headers** — Select articles → instant download

### 3.2 Feature parity with NZBGet
- [ ] Speed graph (canvas chart)
- [ ] Per-server download statistics
- [ ] Scheduler (speed limits by time of day)
- [ ] Notification system
- [ ] Custom post-processing scripts

---

## Implementation Order

1. **Backend: Add group/header tables + API endpoints** (1-2 days)
   - Port SQL and handler code from rustNewsreader
   - Adapt from sqlx to rusqlite
   - Add "download selected" endpoint

2. **Angular: Set up frontend project** (1 day)
   - ng new, Angular Material, proxy config
   - Auth service + guards
   - Core services (API, WebSocket)

3. **Angular: Queue + History views** (1-2 days)
   - Port existing vanilla JS functionality to Angular components
   - Progress bars, speed display, pause/resume

4. **Angular: Groups tab** (1 day)
   - Port from rustNewsreader's newsreader-view component
   - Add checkbox selection + "Download Selected" button

5. **Angular: Settings + RSS + Logs** (1-2 days)
   - Server/category CRUD dialogs
   - RSS feed manager
   - Log viewer

6. **Testing: Playwright E2E** (1 day)
   - Port test infrastructure from rustNewsreader
   - Adapt for rustnzbd's data model

7. **Desktop: Update Tauri wrapper** (0.5 day)
   - Point at new Angular build output

---

## Files to Modify

| File | Change |
|------|--------|
| `crates/nzb-core/src/db.rs` | Add groups, headers, headers_fts tables + CRUD |
| `crates/nzb-core/src/models.rs` | Add GroupRow, HeaderRow, ThreadSummary models |
| `crates/nzb-web/src/server.rs` | Add ~15 new routes |
| `crates/nzb-web/src/handlers.rs` | Add group/header/article handler functions |
| `crates/nzb-web/static/` | Replace with `frontend/dist/` via rust-embed |
| `Cargo.toml` | No changes needed (dependencies already present) |
| `build.rs` | NEW: Run ng build before cargo build |
| `frontend/` | NEW: Entire Angular project |
