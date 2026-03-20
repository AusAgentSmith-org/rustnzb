# rustnzbd — Remaining TODO

Status check against sabnzbd/review.md (March 2026). Items ordered by impact.

---

## Functional Gaps

- [ ] **URL import (addurl)** — SABnzbd compat handler is a stub (returns fake nzo_id, never fetches). Sonarr/Radarr use `addurl` for NZB indexer links. Wire up `reqwest` to actually download the NZB and enqueue it.
- [ ] **Queue reordering** — UI has move-up/move-down buttons that toast "not yet supported". Add `POST /api/queue/{id}/move` (or similar) and implement in `queue_manager.rs`.
- [ ] **Priority change after enqueue** — No endpoint to change a job's priority once it's in the queue. Add `PUT /api/queue/{id}/priority`.
- [ ] **Category CRUD via API** — Only `GET /api/config/categories` exists. Add create/update/delete so the UI and API consumers can manage categories without editing TOML.
- [ ] **Download resume on restart** — Queue persists across restarts but unfinished articles restart from scratch. Consider checkpointing per-file segment progress so partially-downloaded jobs don't re-fetch everything.

## Performance

- [ ] **SIMD yEnc decoder** — Current decoder is scalar (byte-at-a-time loop). For saturating fast connections (>100 MB/s), a SIMD path using `std::simd` or manual intrinsics would help. Low priority unless decode becomes the bottleneck — profile first.

## API / Integration

- [ ] **Swagger UI wiring** — `utoipa` is in deps but verify the `/swagger-ui` route is actually mounted and working. If not, wire it up.
- [ ] **SABnzbd compat coverage** — Audit which `mode=` values Sonarr, Radarr, and Lidarr actually call. The compat layer covers the basics but may be missing edge cases (e.g. `mode=config`, `mode=get_cats`, `mode=change_cat`). Test with real arr instances.

## Operational

- [ ] **Graceful shutdown** — Verify that in-flight downloads are cleanly stopped and queue state is flushed to SQLite on SIGTERM/SIGINT. Important for Docker deployments.
- [ ] **Disk space checks** — Pre-flight check for available disk space before starting a download. Alert or pause if disk is critically low.
- [ ] **Docker health check** — Add a `/api/health` endpoint (or use `/api/status`) for `HEALTHCHECK` in Docker.

## Nice-to-Have (v2 territory per review.md)

These are explicitly deferred in review.md but worth tracking:

- [ ] Directory watching (watch folder for NZB files)
- [ ] RSS feed monitoring
- [ ] File sorting / media renaming (guessit equivalent)
- [ ] Notification system (apprise or similar)
- [ ] External post-processing scripts
- [ ] Scheduling (speed limits by time, pause/resume on schedule)
- [ ] Per-job bandwidth limiting
