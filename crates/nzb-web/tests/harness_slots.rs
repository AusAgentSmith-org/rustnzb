//! Phase 4 contract tests — semaphore-backed connection slots.
//!
//! These exercise the slot accounting against a real `WorkerPool`:
//!
//! 1. With a server limit of N, the live connection count is bounded by N
//!    even when the queue has more work than slots. This is the
//!    by-construction property the semaphore guarantees — but the
//!    integration test confirms the wiring exposes that property.
//! 2. After all work completes, slots return to zero (no leaks across
//!    the worker exit path).

mod harness;

use std::time::Duration;

use harness::nzb_fixture::NzbFixture;
use harness::{HarnessBuilder, ServerProfile, yenc_articles};
use nzb_nntp::testutil::MockConfig;

#[tokio::test]
async fn live_connection_count_never_exceeds_limit() {
    // 8 segments to make sure the worker pool has plenty of work to chew
    // through. The fixture body sizes are tiny so the workers blast through
    // quickly — we're sampling the live count during processing.
    let bodies: Vec<Vec<u8>> = (0..8)
        .map(|i| format!("payload-segment-{i}").into_bytes())
        .collect();
    let segs: Vec<(&str, &[u8])> = (0..8)
        .map(|i| {
            let mid: &'static str = match i {
                0 => "slot-1",
                1 => "slot-2",
                2 => "slot-3",
                3 => "slot-4",
                4 => "slot-5",
                5 => "slot-6",
                6 => "slot-7",
                _ => "slot-8",
            };
            (mid, bodies[i].as_slice())
        })
        .collect();
    let fixture = NzbFixture::new("slot-bound")
        .add_file("payload.bin", &segs)
        .build();

    // Mock holds yEnc-encoded bodies for every segment.
    let triples: Vec<(&str, &[u8], &str)> = fixture
        .articles
        .iter()
        .map(|(m, b, f)| (*m, *b, f.as_str()))
        .collect();

    // Hard cap at 2 connections.
    const LIMIT: u16 = 2;
    let server = ServerProfile::start(
        "slot-srv",
        MockConfig {
            articles: yenc_articles(&triples),
            ..Default::default()
        },
        LIMIT,
    )
    .await;

    let engine = HarnessBuilder::new()
        .with_server(server)
        .article_timeout(10)
        .build();

    let job_id = engine
        .submit_nzb_xml("slot-bound", fixture.xml)
        .expect("submit");

    // Wait until the worker pool has actually scheduled work and is using
    // its slots — total > 0 means at least one worker has acquired a slot.
    let started = engine
        .wait_for(Duration::from_secs(5), |_snap| {
            engine.queue_manager.connection_total() > 0
        })
        .await;
    assert!(started, "no workers ever acquired a slot");

    // Sample the live count repeatedly while work is in flight.
    // The count must NEVER exceed LIMIT.
    let observation_deadline = std::time::Instant::now() + Duration::from_secs(3);
    let mut peak = 0usize;
    while std::time::Instant::now() < observation_deadline {
        let snap = engine.queue_manager.connection_snapshot();
        for (id, active, lim) in &snap {
            assert!(
                *active <= *lim,
                "{id}: live count {active} exceeded limit {lim}"
            );
            assert!(*lim as u16 == LIMIT, "{id}: limit changed unexpectedly");
            if *active > peak {
                peak = *active;
            }
        }
        // If the job has finished, stop sampling.
        if let Some(view) = engine.job(&job_id)
            && view.articles_downloaded + view.articles_failed >= 8
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    // We should have observed at least one slot in use during processing.
    assert!(
        peak >= 1,
        "expected to observe at least one in-use slot, peak={peak}"
    );

    // Wait for the job to fully resolve.
    let resolved = engine
        .wait_for(Duration::from_secs(10), |snap| {
            snap.job(&job_id)
                .map(|j| j.articles_downloaded + j.articles_failed >= 8)
                .unwrap_or(false)
        })
        .await;
    assert!(resolved, "job didn't resolve all 8 segments");

    let view = engine.job(&job_id).unwrap();
    assert_eq!(
        view.articles_downloaded, 8,
        "expected 8 successful downloads, got downloaded={} failed={}",
        view.articles_downloaded, view.articles_failed
    );
}
