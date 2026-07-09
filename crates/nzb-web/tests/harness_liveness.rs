//! Integration test for socket-level liveness heartbeat.
//!
//! The idle-worker watchdog used to key off `last_progress`, which was only
//! ticked on full article decode. Under provider throttling (recv_ms
//! 30-60s), workers fetching a slow article looked idle for the full fetch
//! duration and got false-evicted right at the 60s eviction threshold.
//! This caused a reconnect storm that providers (Frugalusenet especially)
//! treat as DoS behaviour and answer with a session kill — so we got
//! precisely the "downloads hung" symptom users reported.
//!
//! The fix: nzb-nntp 0.2.14+ exposes an `Arc<AtomicU64>` heartbeat that
//! ticks on every successful line read from the socket. nzb-web attaches
//! `last_progress` to that heartbeat, so workers receiving bytes stay
//! "alive" regardless of whether they've completed an article.
//!
//! This test simulates the exact failure mode: a slow mock server where
//! each NNTP response takes longer than the configured idle threshold
//! per article, and asserts the worker is NOT evicted.

mod harness;

use std::time::Duration;

use harness::nzb_fixture::NzbFixture;
use harness::{HarnessBuilder, ServerProfile, yenc_articles};
use nzb_nntp::testutil::MockConfig;
use nzb_web::nzb_core::models::JobStatus;

/// Aggressive: idle threshold 2 s, server adds 500 ms delay per write.
/// A single article fetch pulls multiple lines (welcome, headers, body lines)
/// so the cumulative response time exceeds the 2 s idle threshold. Without
/// the byte-level heartbeat, the supervisor would evict the worker mid-fetch.
/// With the heartbeat, every line tick keeps `last_progress` fresh so the
/// worker rides through the slow article and completes it.
#[tokio::test]
async fn slow_server_does_not_trigger_idle_eviction() {
    // Build a tiny 2-segment NZB — small enough to finish well under any
    // test timeout, big enough that the response-delay cumulates.
    let body1 = b"slow-body-one-segment-content-here-to-add-some-length".to_vec();
    let body2 = b"slow-body-two-segment-content-here-to-add-some-length".to_vec();
    let fixture = NzbFixture::new("slow")
        .add_file(
            "slow.bin",
            &[
                ("slow-1@test", body1.as_slice()),
                ("slow-2@test", body2.as_slice()),
            ],
        )
        .build();

    let triples: Vec<(&str, &[u8], &str)> = fixture
        .articles
        .iter()
        .map(|(m, b, f)| (*m, *b, f.as_str()))
        .collect();
    let articles = yenc_articles(&triples);

    // 500 ms response_delay is applied per write (every line). An NNTP
    // ARTICLE response has welcome/header/body lines, so a single article
    // ends up taking ~2-3 s of wall clock — past the 2 s idle threshold
    // we set below.
    let server = ServerProfile::start(
        "slow-srv",
        MockConfig {
            articles,
            response_delay: Some(Duration::from_millis(500)),
            ..Default::default()
        },
        1,
    )
    .await;

    let engine = HarnessBuilder::new()
        .with_server(server)
        .article_timeout(30)
        .build();

    // Idle threshold shorter than a single article's fetch time.
    // Without the liveness heartbeat, the supervisor would evict during
    // the fetch. With it, byte-level ticks keep the worker alive.
    engine
        .queue_manager
        .set_max_worker_idle(Duration::from_secs(2));

    let job_id = engine
        .submit_nzb_xml("slow", fixture.xml)
        .expect("submit nzb");

    // Give it plenty of wall-clock time (the articles are slow by design).
    let done = engine
        .wait_for(Duration::from_secs(30), |snap| {
            snap.job(&job_id)
                .map(|j| j.articles_downloaded == 2)
                .unwrap_or(false)
        })
        .await;

    let view = engine.job(&job_id).expect("job present");
    let evictions = engine.queue_manager.worker_eviction_count();

    assert!(
        done,
        "slow server job didn't complete: status={} downloaded={} failed={} evictions={}",
        view.status, view.articles_downloaded, view.articles_failed, evictions
    );
    assert_eq!(
        evictions, 0,
        "worker was falsely evicted during slow article fetch ({evictions}x) — \
         liveness heartbeat isn't keeping last_progress fresh during byte reads"
    );
    assert_eq!(view.articles_failed, 0);
}

/// Regression guard: a truly silent post-connect server eventually causes
/// the JOB to abort via `no_progress_timeout`. With byte-level heartbeat,
/// the idle watchdog correctly rides through reconnect cycles (welcome
/// banner reads count as liveness), but the JOB-LEVEL no-progress timer
/// still catches the case where we never complete or fail an article.
///
/// This is the replacement for the previous
/// "worker-level eviction always fires" assertion — under the new model,
/// progress is measured per byte at the worker level, per article at the
/// job level. Zombie detection still works; it's just delegated to the
/// higher-level supervisor.
#[tokio::test]
async fn silent_server_aborts_job_via_no_progress() {
    let body = b"silent-body".to_vec();
    let fixture = NzbFixture::new("silent")
        .add_file("silent.bin", &[("silent-1@test", body.as_slice())])
        .build();

    // Mock goes silent after any ARTICLE command → no article ever
    // completes or fails. Welcome banner + (maybe) auth still respond on
    // reconnect, so the worker keeps cycling but makes no real progress.
    let server = ServerProfile::start(
        "silent-srv",
        MockConfig {
            hang_after_command: Some("ARTICLE".into()),
            ..Default::default()
        },
        2,
    )
    .await;

    let engine = HarnessBuilder::new()
        .with_server(server)
        .article_timeout(3)
        .build();
    // Short no-progress threshold so the test converges quickly. Production
    // default is 300s — for tests we compress the window.
    engine
        .queue_manager
        .set_no_progress_timeout(Duration::from_secs(6));

    let job_id = engine
        .submit_nzb_xml("silent", fixture.xml)
        .expect("submit nzb");

    // Must reach Failed within a reasonable window.
    let failed = engine
        .wait_for_status(&job_id, Duration::from_secs(30), &[JobStatus::Failed])
        .await;
    let view = engine.job(&job_id).expect("job present");
    assert!(
        failed,
        "silent-server job didn't abort: status={} downloaded={} failed={}",
        view.status, view.articles_downloaded, view.articles_failed
    );
}

/// Sanity: the heartbeat-enabled engine still drives a normal (fast) download
/// to completion. Guards against any regression in the happy path.
#[tokio::test]
async fn fast_server_still_completes_normally() {
    let body = b"fast body".to_vec();
    let fixture = NzbFixture::new("fast")
        .add_file("fast.bin", &[("fast-1@test", body.as_slice())])
        .build();

    let triples: Vec<(&str, &[u8], &str)> = fixture
        .articles
        .iter()
        .map(|(m, b, f)| (*m, *b, f.as_str()))
        .collect();
    let server = ServerProfile::start(
        "fast-srv",
        MockConfig {
            articles: yenc_articles(&triples),
            ..Default::default()
        },
        2,
    )
    .await;

    let engine = HarnessBuilder::new()
        .with_server(server)
        .article_timeout(10)
        .build();

    let job_id = engine
        .submit_nzb_xml("fast", fixture.xml)
        .expect("submit nzb");

    let done = engine
        .wait_for_status(&job_id, Duration::from_secs(10), &[JobStatus::Completed])
        .await;

    let view = engine.job(&job_id).expect("job present");
    assert!(
        done,
        "fast download regressed: status={} downloaded={} failed={}",
        view.status, view.articles_downloaded, view.articles_failed
    );
}
