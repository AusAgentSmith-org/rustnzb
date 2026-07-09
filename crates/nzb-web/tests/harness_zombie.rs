//! Canary test — proves the zombie-worker bug is fixed.
//!
//! ## What this test asserts
//!
//! When all configured providers go silent mid-download (the production
//! symptom: provider session killed after a stall reconnect storm, sockets
//! still alive at the TCP layer but no NNTP responses ever arriving), the
//! pool must eventually detect there's no real progress and abort the job
//! — workers do not hold slots indefinitely.
//!
//! ## History
//!
//! Before Phase 5, this test failed (see git history): the
//! `articles_downloaded=0, articles_failed=0` zombie state was permanent
//! because workers cycled `next_work_item → pop_workable → sleep 500ms`
//! forever and the only stall detection was a per-article `tokio::time::
//! timeout` that triggered a same-server reconnect rather than eviction.
//! Phase 5 added an explicit `last_progress` heartbeat per worker plus a
//! 1-second supervisor tick that evicts workers idle for too long.
//!
//! ## Update (byte-level liveness heartbeat)
//!
//! The idle watchdog's liveness signal is now socket-byte-level, not
//! article-completion-level. This fixes false eviction of slow-but-working
//! workers (provider throttling can make recv_ms approach the idle
//! threshold per article). With byte-level ticks, the ARTICLE-hang case
//! gets caught by per-article stall timeout → reconnect → welcome banner
//! ticks heartbeat → ... so the worker-level watchdog correctly doesn't
//! fire. Zombie detection is now delegated to the JOB-LEVEL
//! `no_progress_timeout`, which observes that no articles ever complete
//! or definitively fail. This test now verifies the job abort path.

mod harness;

use std::time::Duration;

use harness::nzb_fixture::NzbFixture;
use harness::{HarnessBuilder, ServerProfile};
use nzb_nntp::testutil::MockConfig;
use nzb_web::nzb_core::models::JobStatus;

#[tokio::test]
async fn hang_on_article_does_not_strand_workers() {
    // Five segments so that workers fill the pipeline and have something
    // in flight when the hang fires.
    let bodies: Vec<Vec<u8>> = (0..5).map(|i| format!("body-{i}").into_bytes()).collect();
    let segs: Vec<(&str, &[u8])> = (0..5)
        .map(|i| {
            let mid: &'static str = match i {
                0 => "z-1@test",
                1 => "z-2@test",
                2 => "z-3@test",
                3 => "z-4@test",
                _ => "z-5@test",
            };
            (mid, bodies[i].as_slice())
        })
        .collect();

    let fixture = NzbFixture::new("zombie")
        .add_file("zombie.bin", &segs)
        .build();

    // Mock server: stop responding the first time it sees ARTICLE.
    // No yEnc bodies needed — the worker never gets a successful response.
    let server = ServerProfile::start(
        "hang-srv",
        MockConfig {
            hang_after_command: Some("ARTICLE".into()),
            ..Default::default()
        },
        2,
    )
    .await;

    // Short article_timeout so the per-article stall trips quickly, and a
    // short no_progress_timeout so the job-level watchdog converges in
    // seconds rather than the production default of 300s. With byte-level
    // liveness ticks, worker-level eviction doesn't fire here (reconnect
    // welcome reads keep the heartbeat fresh) — this is intentional, and
    // the no-progress path is what actually catches the zombie.
    let engine = HarnessBuilder::new()
        .with_server(server)
        .article_timeout(2)
        .build();
    engine
        .queue_manager
        .set_no_progress_timeout(Duration::from_secs(6));

    let job_id = engine
        .submit_nzb_xml("zombie", fixture.xml)
        .expect("submit nzb");

    // First confirm the job actually entered Downloading and workers have
    // begun holding slots — otherwise we'd be measuring the wrong thing.
    let downloading = engine
        .wait_for_status(&job_id, Duration::from_secs(5), &[JobStatus::Downloading])
        .await;
    assert!(downloading, "job never reached Downloading");

    let workers_started = engine
        .wait_for(Duration::from_secs(5), |_| {
            engine.queue_manager.connection_total() > 0
        })
        .await;
    assert!(workers_started, "workers never connected");

    // Load-bearing assertion: the job must exit Downloading via the
    // no-progress watchdog within a reasonable window. If it stays in
    // Downloading forever, workers are stranded.
    let resolved = engine
        .wait_for_status(
            &job_id,
            Duration::from_secs(25),
            &[JobStatus::Failed, JobStatus::Completed],
        )
        .await;

    let final_view = engine.job(&job_id).expect("job present");
    assert!(
        resolved,
        "ZOMBIE: job never left Downloading — no-progress watchdog didn't fire. \
         status={}, downloaded={}, failed={}",
        final_view.status, final_view.articles_downloaded, final_view.articles_failed
    );
}
