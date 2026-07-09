//! Smoke test for the integration harness — proves the harness can spin up a
//! real `QueueManager` against a `MockNntpServer` and submit an NZB end to end.

mod harness;

use std::collections::HashMap;
use std::time::Duration;

use harness::nzb_fixture::NzbFixture;
use harness::{HarnessBuilder, ServerProfile, yenc_articles};
use nzb_nntp::testutil::MockConfig;

#[tokio::test]
async fn submitting_an_nzb_advances_to_downloading() {
    // 1) Hand-build a tiny one-file, one-segment NZB.
    let body = b"hello world";
    let fixture = NzbFixture::new("smoke")
        .add_file("hello.txt", &[("smoke-msg-1@test", body.as_slice())])
        .build();

    // 2) yEnc-encode the body and feed it into the mock as the article
    //    payload (so when the worker fetches the message-id, it gets
    //    valid yEnc).
    let triples: Vec<(&str, &[u8], &str)> = fixture
        .articles
        .iter()
        .map(|(m, b, f)| (*m, *b, f.as_str()))
        .collect();
    let articles = yenc_articles(&triples);

    let server = ServerProfile::start(
        "smoke-srv",
        MockConfig {
            articles,
            ..Default::default()
        },
        4,
    )
    .await;

    // 3) Spin up the engine.
    let engine = HarnessBuilder::new()
        .with_server(server)
        .article_timeout(10)
        .build();

    // 4) Submit and verify it lands in the queue.
    let job_id = engine
        .submit_nzb_xml("smoke", fixture.xml)
        .expect("submit nzb");

    // The job must reach Downloading within a few seconds (worker pool
    // claims it from the queue almost immediately).
    // Wait until the article is *resolved* (downloaded or failed), not
    // merely until the job entered Downloading. This ensures the test is
    // actually exercising the decode + assemble pipeline.
    let resolved = engine
        .wait_for(Duration::from_secs(15), |snap| {
            snap.job(&job_id)
                .map(|j| j.articles_downloaded + j.articles_failed >= 1)
                .unwrap_or(false)
        })
        .await;

    let view = engine.job(&job_id).expect("job present");
    eprintln!(
        "SMOKE STATE: status={} downloaded={} failed={} bytes_dl={} total_bytes={}",
        view.status,
        view.articles_downloaded,
        view.articles_failed,
        view.downloaded_bytes,
        view.total_bytes
    );
    assert!(
        resolved,
        "article did not resolve within 15s: status={}, downloaded={}, failed={}",
        view.status, view.articles_downloaded, view.articles_failed
    );
    assert_eq!(view.article_count, 1, "expected 1 article in fixture");
    assert_eq!(
        view.articles_downloaded, 1,
        "expected the article to download successfully (downloaded={}, failed={})",
        view.articles_downloaded, view.articles_failed
    );
}

#[tokio::test]
async fn empty_engine_has_no_jobs() {
    let server = ServerProfile::start("empty", MockConfig::default(), 1).await;
    let engine = HarnessBuilder::new().with_server(server).build();
    let snap = engine.snapshot();
    assert!(snap.jobs.is_empty(), "fresh engine should have no jobs");
}

#[tokio::test]
async fn yenc_articles_helper_encodes_each_body() {
    let triples: Vec<(&str, &[u8], &str)> = vec![
        ("a", b"first body" as &[u8], "a.bin"),
        ("b", b"second body" as &[u8], "b.bin"),
    ];
    let map: HashMap<String, Vec<u8>> = yenc_articles(&triples);
    assert_eq!(map.len(), 2);
    // yEnc-encoded bodies start with "=ybegin"
    for v in map.values() {
        assert!(
            v.starts_with(b"=ybegin"),
            "expected yEnc header, got: {:?}",
            String::from_utf8_lossy(&v[..30.min(v.len())])
        );
    }
}
