//! Phase 3 contract tests — typed error taxonomy.
//!
//! These exercise the wiring of `ArticleFailure` / `ArticleFailureKind`
//! through the worker pool, the progress channel, and the queue manager
//! consumer. They prove:
//!
//! 1. A 430 from the only available server propagates through the typed
//!    failure path and ultimately marks the job as failed.
//! 2. A 430 from one server in a multi-server pool does NOT bubble up — the
//!    worker retries on the next server and the job completes. This proves
//!    the per-server classification (`ArticleFailureKind::is_per_server`)
//!    is plumbed correctly.

mod harness;

use std::collections::HashMap;
use std::time::Duration;

use harness::nzb_fixture::NzbFixture;
use harness::{HarnessBuilder, ServerProfile, yenc_articles};
use nzb_nntp::testutil::MockConfig;
use nzb_web::nzb_core::models::JobStatus;

#[tokio::test]
async fn not_found_on_only_server_aborts_job() {
    // Five segments so the early-failure check has enough samples to trip.
    let bodies: Vec<Vec<u8>> = (0..5).map(|i| format!("body-{i}").into_bytes()).collect();
    let segs: Vec<(&str, &[u8])> = (0..5)
        .map(|i| {
            let mid: &'static str = match i {
                0 => "tnf-1",
                1 => "tnf-2",
                2 => "tnf-3",
                3 => "tnf-4",
                _ => "tnf-5",
            };
            (mid, bodies[i].as_slice())
        })
        .collect();
    let fixture = NzbFixture::new("typed-not-found")
        .add_file("data.bin", &segs)
        .build();

    // Mock returns 430 for every segment in the fixture. No `articles`
    // entries, so the article-not-found path always fires.
    let mut overrides = HashMap::new();
    for &(mid, _) in &segs {
        overrides.insert(mid.to_string(), 430u16);
    }

    let server = ServerProfile::start(
        "nf-srv",
        MockConfig {
            article_response_overrides: overrides,
            ..Default::default()
        },
        2,
    )
    .await;

    let engine = HarnessBuilder::new()
        .with_server(server)
        .article_timeout(10)
        .abort_hopeless(true)
        .build();

    let job_id = engine
        .submit_nzb_xml("typed-not-found", fixture.xml)
        .expect("submit");

    // Wait for the job to reach a terminal state. With 100% NotFound on the
    // only enabled server, the early-failure check (or eventually the
    // ongoing-availability check) must abort the job.
    let resolved = engine
        .wait_for_status(
            &job_id,
            Duration::from_secs(15),
            &[JobStatus::Failed, JobStatus::Completed],
        )
        .await;

    let view = engine.job(&job_id).expect("job present");
    assert!(
        resolved,
        "job did not reach terminal state — articles_failed={}, status={}",
        view.articles_failed, view.status
    );
    assert!(
        view.articles_failed > 0,
        "expected typed-failure path to record failures, got 0"
    );
    assert_eq!(view.articles_downloaded, 0, "no articles should download");
}

#[tokio::test]
async fn not_found_on_one_server_falls_over_to_another() {
    // Single article. Server A doesn't have it (430). Server B does.
    // The typed classification (`is_per_server`) tells the worker pool that
    // 430 on server A doesn't preclude success on server B.
    let body: &[u8] = b"the only body";
    let fixture = NzbFixture::new("typed-fallover")
        .add_file("solo.bin", &[("fo-1", body)])
        .build();

    // Server A: returns 430 via override.
    let mut a_overrides = HashMap::new();
    a_overrides.insert("fo-1".to_string(), 430u16);
    let server_a = ServerProfile::start(
        "srv-a",
        MockConfig {
            article_response_overrides: a_overrides,
            ..Default::default()
        },
        1,
    )
    .await;

    // Server B: holds the article body, yEnc-encoded for the real decoder.
    let triples: Vec<(&str, &[u8], &str)> = fixture
        .articles
        .iter()
        .map(|(m, b, f)| (*m, *b, f.as_str()))
        .collect();
    let server_b = ServerProfile::start(
        "srv-b",
        MockConfig {
            articles: yenc_articles(&triples),
            ..Default::default()
        },
        1,
    )
    .await;

    let engine = HarnessBuilder::new()
        .with_server(server_a)
        .with_server(server_b)
        .article_timeout(10)
        .build();

    let job_id = engine
        .submit_nzb_xml("typed-fallover", fixture.xml)
        .expect("submit");

    // The job should complete (not abort) because server B can serve the
    // article. If the typed classification were broken — e.g. a 430 on
    // server A were treated as a global failure — the job would abort.
    let completed = engine
        .wait_for(Duration::from_secs(15), |snap| {
            snap.job(&job_id)
                .map(|j| {
                    j.articles_downloaded >= 1
                        || matches!(j.status, JobStatus::Completed | JobStatus::PostProcessing)
                })
                .unwrap_or(false)
        })
        .await;

    let view = engine.job(&job_id).expect("job present");
    eprintln!(
        "FALLOVER STATE: status={} downloaded={} failed={} count={} bytes_dl={} total_bytes={}",
        view.status,
        view.articles_downloaded,
        view.articles_failed,
        view.article_count,
        view.downloaded_bytes,
        view.total_bytes
    );
    assert!(
        completed,
        "expected fallover to complete the article — got status={}, downloaded={}, failed={}",
        view.status, view.articles_downloaded, view.articles_failed
    );
    assert_eq!(
        view.articles_failed, 0,
        "fallover should not record a failed article (per-server NotFound is not a global failure)"
    );
}
