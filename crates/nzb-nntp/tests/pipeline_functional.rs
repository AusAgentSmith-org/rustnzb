//! Functional tests for NNTP pipelining using the in-process mock server.

use std::collections::HashMap;

use nzb_nntp::connection::NntpConnection;
use nzb_nntp::pipeline::Pipeline;
use nzb_nntp::testutil::{MockConfig, MockNntpServer, test_config};

#[tokio::test]
async fn test_pipeline_multiple_articles() {
    let mut articles = HashMap::new();
    for i in 0..5 {
        articles.insert(
            format!("art-{i}"),
            format!("Body of article {i}").into_bytes(),
        );
    }

    let server = MockNntpServer::start(MockConfig {
        articles,
        ..Default::default()
    })
    .await;

    let config = test_config(server.port());
    let mut conn = NntpConnection::new("test".into());
    conn.connect(&config).await.unwrap();

    let mut pipeline = Pipeline::new(5);
    for i in 0..5 {
        pipeline.submit(format!("art-{i}"), i);
    }

    let results = pipeline.process_all(&mut conn).await.unwrap();
    assert_eq!(results.len(), 5, "should receive 5 results");

    for result in &results {
        assert!(
            result.result.is_ok(),
            "article {} should succeed: {:?}",
            result.request.message_id,
            result.result.as_ref().err()
        );
    }

    conn.quit().await.unwrap();
}

#[tokio::test]
async fn test_pipeline_mixed_hit_miss() {
    let mut articles = HashMap::new();
    articles.insert("hit-0".to_string(), b"found 0".to_vec());
    articles.insert("hit-1".to_string(), b"found 1".to_vec());
    articles.insert("hit-2".to_string(), b"found 2".to_vec());
    // miss-0, miss-1 are NOT in the mock → 430

    let server = MockNntpServer::start(MockConfig {
        articles,
        ..Default::default()
    })
    .await;

    let config = test_config(server.port());
    let mut conn = NntpConnection::new("test".into());
    conn.connect(&config).await.unwrap();

    let mut pipeline = Pipeline::new(5);
    pipeline.submit("hit-0".into(), 0);
    pipeline.submit("miss-0".into(), 1);
    pipeline.submit("hit-1".into(), 2);
    pipeline.submit("miss-1".into(), 3);
    pipeline.submit("hit-2".into(), 4);

    let results = pipeline.process_all(&mut conn).await.unwrap();
    assert_eq!(results.len(), 5);

    // Verify correct hit/miss per article
    let hits: Vec<_> = results.iter().filter(|r| r.result.is_ok()).collect();
    let misses: Vec<_> = results.iter().filter(|r| r.result.is_err()).collect();
    assert_eq!(hits.len(), 3, "3 articles should be found");
    assert_eq!(misses.len(), 2, "2 articles should be missing");

    conn.quit().await.unwrap();
}

#[tokio::test]
async fn test_pipeline_depth_1() {
    // depth=1 means no pipelining — send one, wait for response, send next.
    let mut articles = HashMap::new();
    articles.insert("seq-0".to_string(), b"data 0".to_vec());
    articles.insert("seq-1".to_string(), b"data 1".to_vec());
    articles.insert("seq-2".to_string(), b"data 2".to_vec());

    let server = MockNntpServer::start(MockConfig {
        articles,
        ..Default::default()
    })
    .await;

    let config = test_config(server.port());
    let mut conn = NntpConnection::new("test".into());
    conn.connect(&config).await.unwrap();

    let mut pipeline = Pipeline::new(1); // depth = 1
    pipeline.submit("seq-0".into(), 0);
    pipeline.submit("seq-1".into(), 1);
    pipeline.submit("seq-2".into(), 2);

    let results = pipeline.process_all(&mut conn).await.unwrap();
    assert_eq!(results.len(), 3);
    for r in &results {
        assert!(
            r.result.is_ok(),
            "all articles should succeed: {:?}",
            r.result.as_ref().err()
        );
    }

    conn.quit().await.unwrap();
}
